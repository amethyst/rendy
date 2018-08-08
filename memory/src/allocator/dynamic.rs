
use std::{ops::Range, ptr::NonNull};
use veclist::VecList;


use allocator::Allocator;
use block::Block;
use device::Device;
use error::*;
use map::*;
use memory::*;
use util;

pub struct DynamicBlock<T> {
    index: usize,
    memory: *const Memory<T>,
    mapping: NonNull<u8>,
    range: Range<u64>,
}

impl<T> DynamicBlock<T> {
    fn shared_memory(&self) -> &Memory<T> {
        unsafe { // Memory can't be freed until all chunks.
            &*self.memory
        }
    }
}

impl<T> Block<T> for DynamicBlock<T> {
    fn properties(&self) -> Properties {
        self.shared_memory().properties()
    }

    fn memory(&self) -> &T {
        self.shared_memory().raw()
    }

    fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    fn map<'a, D>(&'a mut self, device: &D, range: Range<u64>) -> Result<MappedRange<'a, T>, MappingError>
    where
        D: Device<T>,
    {
        // Check if specified range is not out of block bounds.
        if range.end < range.start || range.end > (self.range.end - self.range.start) {
            return Err(MappingError::OutOfBounds);
        }

        let start = range.start + self.range.start;
        let end = range.end + self.range.start;

        debug_assert!(util::fits_in_usize(start) && util::fits_in_usize(end), "Implied by check above because arena memory size must fit in usize");

        let coherent = maybe_coherent(self.shared_memory().host_coherent());

        Ok(unsafe {
            MappedRange {
                ptr: NonNull::new_unchecked(self.mapping.as_ptr().add(start as usize)),
                memory: self.shared_memory().raw(),
                offset: start,
                length: (end - start) as usize,
                coherent,
            }
        })
    }

    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<T>,
    {
    }
}

/// Block allocated for chunk.
enum Chunk<T> {
    /// Allocated from device.
    Dedicated(Box<Memory<T>>, NonNull<u8>),

    /// Allocated from chunk of bigger blocks.
    Dynamic(DynamicBlock<T>),
}

impl<T> Chunk<T> {
    fn shared_memory(&self) -> &Memory<T> {
        match self {
            Chunk::Dedicated(boxed, _) => &*boxed,
            Chunk::Dynamic(chunk_block) => chunk_block.shared_memory(),
        }
    }

    fn range(&self) -> Range<u64> {
        match self {
            Chunk::Dedicated(boxed, _) => 0 .. boxed.size(),
            Chunk::Dynamic(chunk_block) => chunk_block.range(),
        }
    }

    fn mapping(&self) -> NonNull<u8> {
        match self {
            Chunk::Dedicated(_, mapping) => *mapping,
            Chunk::Dynamic(chunk_block) => chunk_block.mapping,
        }
    }
}

/// List of chunks
struct Size<T> {
    /// List of chunks.
    chunks: VecList<Chunk<T>>,

    /// Number of elements in `chunks`.
    chunks_count: usize,

    /// Top level mask over `chunks`.
    top_mask: u64,

    /// Bitset with no vacant blocks chunks are `0`s and with vancant blocks are `1`s.
    chunks_mask: [u64; 64],

    /// Bitset with occupied blocks are `0`s and vacant blocks are `1`s.
    blocks_mask: [u64; 4096],
}

pub struct DynamicAllocator<T> {
    /// Memory type that this allocator allocates.
    memory_type: u32,

    /// Memory properties of the memory type.
    memory_properties: Properties,

    /// Minimal block size.
    /// Any request less than this will be answered with block of this size.
    block_size_granularity: u64,

    /// List of chunk lists.
    /// Each index corresponds to `block_size_granularity * index` size.
    sizes: Vec<Size<T>>,
}

impl<T> DynamicAllocator<T> {
    /// Maximum block size.
    /// Any request bigger will be answered with `Err(OutOfMemoryError::OutOfDeviceMemory)`.
    pub fn max_block_size(&self) -> u64 {
        self.block_size_granularity * self.sizes.len() as u64
    }

    /// Returns size index.
    fn size_index(&self, size: u64) -> usize {
        assert!(size <= self.max_block_size());
        ((size - 1) / self.block_size_granularity) as usize + 1
    }

    /// Get block size for the size index.
    fn block_size(&self, index: usize) -> u64 {
        self.block_size_granularity * index as u64
    }

    /// Allocate super-block to use as chunk memory.
    fn alloc_chunk<D>(&mut self, device: &D, size_index: usize) -> Result<(Chunk<T>, u64), MemoryError>
    where
        D: Device<T>,
    {
        debug_assert!(self.memory_properties.host_visible());

        if size_index >= self.sizes.len() {
            let size = self.block_size(size_index);
            let (memory, mapping) = unsafe { // Valid memory type specified.
                let raw = device.allocate(self.memory_type, size)?;
                let mapping = match device.map(&raw, 0 .. size) {
                    Ok(mapping) => mapping,
                    Err(error) => {
                        device.free(raw);
                        return Err(error.into())
                    }
                };
                let memory = Memory::from_raw(raw, size, self.memory_properties);
                (memory, mapping)
            };
            Ok((Chunk::Dedicated(Box::new(memory), mapping), size))
        } else {
            let (dynamic_block, allocated) = self.alloc_from_chunk(device, size_index)?;
            Ok((Chunk::Dynamic(dynamic_block), allocated))
        }
    }

    /// Allocate super-block to use as chunk memory.
    fn free_chunk<D>(&mut self, device: &D, chunk: Chunk<T>) -> u64
    where
        D: Device<T>,
    {
        match chunk {
            Chunk::Dedicated(boxed, _) => {
                let size = boxed.size();
                unsafe {
                    device.unmap(boxed.raw());
                    device.free(boxed.into_raw());
                }
                size
            }
            Chunk::Dynamic(dynamic_block) => {
                self.free(device, dynamic_block)
            }
        }
    }

    /// Allocate from chunk.
    fn alloc_from_chunk<D>(&mut self, device: &D, size_index: usize) -> Result<(DynamicBlock<T>, u64), MemoryError>
    where
        D: Device<T>,
    {
        let (ix, allocated) = if self.sizes[size_index].top_mask == 0 {
            if self.sizes[size_index].chunks_count == 4096 {
                // Can't allocate more.
                return Err(OutOfMemoryError::OutOfDeviceMemory.into());
            }

            // Allocate new chunk.
            let chunk_size = self.block_size(size_index) * 64;
            let chunk_size_index = self.size_index(chunk_size);
            let (chunk, allocated) = self.alloc_chunk(device, chunk_size_index)?;

            self.sizes[size_index].chunks_count += 1;

            let ref mut size_chunks = self.sizes[size_index];

            let chunk_index = size_chunks.chunks.push(chunk);
            let ix = split_index(chunk_index * 64);

            size_chunks.top_mask |= ix.mask_bit;
            size_chunks.chunks_mask[ix.mask_index] |= ix.chunk_bit;
            size_chunks.blocks_mask[ix.chunk_index] = !ix.block_bit;

            (ix, allocated)
        } else {
            let ref mut size_chunks = self.sizes[size_index];
            let mask_index = size_chunks.top_mask.trailing_zeros() as usize;
            debug_assert!(mask_index < 64);

            let chunk_index = size_chunks.chunks_mask[mask_index].trailing_zeros() as usize;
            debug_assert!(chunk_index < 4096);

            let block_index = size_chunks.blocks_mask[mask_index * 64 | chunk_index].trailing_zeros() as usize;
            debug_assert!(block_index < 262144);

            let ix = make_index(mask_index, chunk_index, block_index);

            size_chunks.blocks_mask[ix.chunk_index] &= !ix.block_bit;
            if size_chunks.blocks_mask[ix.chunk_index] == 0 {
                size_chunks.chunks_mask[ix.mask_index] &= !ix.chunk_bit;
                if size_chunks.chunks_mask[ix.mask_index] == 0 {
                    size_chunks.top_mask &= !ix.mask_bit;
                }
            }

            (ix, 0)
        };

        let ref chunk = self.sizes[size_index].chunks[ix.chunk_index];
        let chunk_range = chunk.range();
        let block_size = self.block_size(size_index);
        let block_offset = chunk_range.start + (ix.block_index % 64) as u64 * block_size;
        let block_range = block_offset .. block_offset + block_size;
        debug_assert!(block_range.end <= chunk_range.end);

        Ok((DynamicBlock {
            range: block_range,
            memory: chunk.shared_memory(),
            index: ix.block_index,
            mapping: chunk.mapping(),
        }, allocated))
    }
}

impl<T> Allocator<T> for DynamicAllocator<T> {
    type Block = DynamicBlock<T>;

    fn alloc<D>(&mut self, device: &D, size: u64, align: u64) -> Result<(DynamicBlock<T>, u64), MemoryError>
    where
        D: Device<T>,
    {
        use std::cmp::max;
        let size_index = self.size_index(max(size, align));

        if size_index >= self.sizes.len() {
            // Too big block requested.
            Err(OutOfMemoryError::OutOfDeviceMemory.into())
        } else {
            self.alloc_from_chunk(device, size_index)
        }
    }

    fn free<D>(&mut self, device: &D, block: DynamicBlock<T>) -> u64
    where
        D: Device<T>,
    {
        let size_index = self.size_index(block.range.end - block.range.start);
        let ix = split_index(block.index);

        self.sizes[size_index].blocks_mask[ix.chunk_index] |= ix.block_bit;
        if self.sizes[size_index].blocks_mask[ix.chunk_index] == !0 {
            self.sizes[size_index].chunks_mask[ix.mask_index] &= !ix.chunk_bit;
            if self.sizes[size_index].chunks_mask[ix.mask_index] == 0 {
                self.sizes[size_index].top_mask &= ix.mask_bit;
            }
            let chunk = self.sizes[size_index].chunks.pop(ix.chunk_index).expect("Block from chunk implies there is a chunk");
            self.free_chunk(device, chunk)
        } else {
            self.sizes[size_index].chunks_mask[ix.mask_index] |= ix.chunk_bit;
            self.sizes[size_index].top_mask |= ix.mask_bit;
            0
        }
    }
}

struct Ix {
    mask_index: usize,
    mask_bit: u64,
    chunk_index: usize,
    chunk_bit: u64,
    block_index: usize,
    block_bit: u64,
}

fn make_index(mask_index: usize, chunk_index: usize, block_index: usize) -> Ix {
    debug_assert!(mask_index < 64);
    debug_assert!(chunk_index < 64);
    debug_assert!(block_index < 64);

    let mask_bit = 1 << mask_index;
    let chunk_index = mask_index * 64 | chunk_index;
    let chunk_bit = 1 << chunk_index;
    let block_index = chunk_index * 64 | block_index;
    let block_bit = 1 << block_index;
    Ix {
        mask_index,
        mask_bit,
        chunk_index,
        chunk_bit,
        block_index,
        block_bit,
    }
}

fn split_index(index: usize) -> Ix {
    make_index(index / 4096, (index / 64) % 64, index % 64)
}
