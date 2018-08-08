
use std::ops::Range;
use veclist::VecList;


use allocator::Allocator;
use block::Block;
use device::Device;
use error::*;
use map::*;
use memory::*;

pub struct ChunkBlock<T> {
    memory: *const Memory<T>,
    range: Range<u64>,
    info: BlockInfo,
}

struct BlockInfo {
    /// Index of chunk in `Chunks`.
    chunk_index: usize,
    /// Index of block in `Chunk`.
    block_index: usize,
}

impl<T> ChunkBlock<T> {
    fn shared_memory(&self) -> &Memory<T> {
        unsafe { // Memory can't be freed until all chunks.
            &*self.memory
        }
    }
}

impl<T> Block<T> for ChunkBlock<T> {
    fn properties(&self) -> Properties {
        self.shared_memory().properties()
    }

    fn memory(&self) -> &T {
        self.shared_memory().raw()
    }

    fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    fn map<'a, D>(&'a mut self, _: &D, _: Range<u64>) -> Result<MappedRange<'a, T>, MappingError> {
        if self.shared_memory().host_visible() {
            Err(MappingError::MappingUnsafe)
        } else {
            Err(MappingError::HostInvisible)
        }
    }

    fn unmap<D>(&mut self, _: &D, _: Range<u64>) {}
}

/// Block allocated for chunk.
enum Chunk<T> {
    /// Allocated from device.
    Dedicated(Box<Memory<T>>),

    /// Allocated from chunk of bigger blocks.
    Chunk(ChunkBlock<T>),
}

impl<T> Chunk<T> {
    fn shared_memory(&self) -> &Memory<T> {
        match self {
            Chunk::Dedicated(boxed) => &*boxed,
            Chunk::Chunk(chunk_block) => chunk_block.shared_memory(),
        }
    }

    fn range(&self) -> Range<u64> {
        match self {
            Chunk::Dedicated(boxed) => 0 .. boxed.size(),
            Chunk::Chunk(chunk_block) => chunk_block.range(),
        }
    }
}

/// List of chunks
struct Size<T> {
    /// List of chunks.
    chunks: VecList<Chunk<T>>,

    /// List of free blocks.
    free: Vec<BlockInfo>,
}

pub struct ChunkAllocator<T> {
    /// Memory type that this allocator allocates.
    memory_type: u32,

    /// Memory properties of the memory type.
    memory_properties: Properties,

    /// Number of blocks per chunk.
    blocks_per_chunk: u32,

    /// Minimal block size.
    /// Any request less than this will be answered with block of this size.
    min_block_size: u64,

    /// List of chunk lists.
    /// Each index corresponds to `min_block_size << index` size.
    sizes: Vec<Size<T>>,
}

impl<T> ChunkAllocator<T> {
    /// Maximum block size.
    /// Any request bigger will be answered with `Err(OutOfMemoryError::OutOfDeviceMemory)`.
    pub fn max_block_size(&self) -> u64 {
        debug_assert!(self.sizes.len() > 0, "Checked on construction");
        self.min_block_size << (self.sizes.len() - 1)
    }

    /// Returns size index.
    fn size_index(&self, size: u64) -> usize {
        64 - ((size - 1) / self.min_block_size).leading_zeros() as usize
    }

    /// Get block size for ther size index.
    fn block_size(&self, index: usize) -> u64 {
        (self.min_block_size << index)
    }

    /// Allocate super-block to use as chunk memory.
    fn alloc_chunk<D>(&mut self, device: &D, size_index: usize) -> Result<(Chunk<T>, u64), MemoryError>
    where
        D: Device<T>,
    {
        if size_index >= self.sizes.len() {
            let size = self.block_size(size_index);
            let memory = unsafe { // Valid memory type specified.
                let memory = device.allocate(self.memory_type, size)?;
                Memory::from_raw(memory, size, self.memory_properties)
            };
            Ok((Chunk::Dedicated(Box::new(memory)), size))
        } else {
            let (chunk_block, allocated) = self.alloc_from_chunk(device, size_index)?;
            Ok((Chunk::Chunk(chunk_block), allocated))
        }
    }

    /// Allocate from chunk.
    fn alloc_from_chunk<D>(&mut self, device: &D, size_index: usize) -> Result<(ChunkBlock<T>, u64), MemoryError>
    where
        D: Device<T>,
    {
        let (block_info, allocated) = if self.sizes[size_index].free.is_empty() {
            // Allocate new chunk.
            let chunk_size = self.block_size(size_index) * self.blocks_per_chunk as u64;
            let chunk_size_index = self.size_index(chunk_size);
            let (chunk, allocated) = self.alloc_chunk(device, chunk_size_index)?;

            let chunk_index = self.sizes[size_index].chunks.push(chunk);

            self.sizes[size_index].free.extend((1 .. self.blocks_per_chunk as usize).map(|block_index| {
                BlockInfo {
                    chunk_index,
                    block_index,
                }
            }));
            (BlockInfo { chunk_index, block_index: 0 }, allocated)
        } else {
            (self.sizes[size_index].free.pop().unwrap(), 0)
        };

        let block_size = self.block_size(size_index);
        let ref chunk = self.sizes[size_index].chunks[block_info.chunk_index];
        let chunk_range = chunk.range();
        let block_range = chunk_range.start + block_info.block_index as u64 * block_size .. chunk_range.start + (block_info.block_index as u64 + 1) * block_size;
        debug_assert!(block_range.end <= chunk_range.end);

        Ok((ChunkBlock {
            range: block_range,
            memory: chunk.shared_memory(),
            info: block_info,
        }, allocated))
    }
}

impl<T> Allocator<T> for ChunkAllocator<T> {
    type Block = ChunkBlock<T>;

    fn alloc<D>(&mut self, device: &D, size: u64, align: u64) -> Result<(ChunkBlock<T>, u64), MemoryError>
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

    fn free<D>(&mut self, _: &D, block: ChunkBlock<T>) -> u64 {
        let size_index = self.size_index(block.range.end - block.range.start);
        self.sizes[size_index].free.push(block.info);
        0
    }
}
