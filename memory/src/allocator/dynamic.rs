use std::{ops::Range, ptr::NonNull};

use ash::{
    version::DeviceV1_0,
    vk::{DeviceMemory, MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags},
};
use hibitset::{BitSet, BitSetLike};
use relevant::Relevant;
use veclist::VecList;

use allocator::Allocator;
use block::Block;
use error::*;
use mapping::*;
use memory::*;
use util::*;

/// Memory block allocated from `DynamicAllocator`
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DynamicBlock {
    index: u32,
    // #[derivative(Debug(format_with = "super::memory_ptr_fmt"))]
    memory: *const Memory,
    ptr: Option<NonNull<u8>>,
    range: Range<u64>,
    #[derivative(Debug = "ignore")]
    relevant: Relevant,
}

unsafe impl Send for DynamicBlock {}
unsafe impl Sync for DynamicBlock {}

impl DynamicBlock {
    fn shared_memory(&self) -> &Memory {
        // Memory won't be freed until last block created from it deallocated.
        unsafe { &*self.memory }
    }

    fn size(&self) -> u64 {
        self.range.end - self.range.start
    }

    fn dispose(self) {
        self.relevant.dispose();
    }
}

impl Block for DynamicBlock {
    #[inline]
    fn properties(&self) -> MemoryPropertyFlags {
        self.shared_memory().properties()
    }

    #[inline]
    fn memory(&self) -> DeviceMemory {
        self.shared_memory().raw()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    #[inline]
    fn map<'a>(
        &'a mut self,
        _device: &impl DeviceV1_0,
        range: Range<u64>,
    ) -> Result<MappedRange<'a>, MappingError> {
        assert!(
            range.start <= range.end,
            "Memory mapping region must have valid size"
        );
        if !self.shared_memory().host_visible() {
            return Err(MappingError::HostInvisible);
        }

        if let Some(ptr) = self.ptr {
            if let Some((ptr, range)) = mapped_sub_range(ptr, self.range.clone(), range) {
                let mapping = unsafe { MappedRange::from_raw(self.shared_memory(), ptr, range) };
                Ok(mapping)
            } else {
                Err(MappingError::OutOfBounds)
            }
        } else {
            Err(MappingError::MappingUnsafe)
        }
    }

    #[inline]
    fn unmap(&mut self, _device: &impl DeviceV1_0) {}
}

/// Config for `DynamicAllocator`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DynamicConfig {
    /// Number of blocks per chunk.
    /// It is advised to keep this equal to bits count in `usize`.
    pub blocks_per_chunk: u32,

    /// All requests are rounded up to multiple of this value.
    pub block_size_granularity: u64,

    /// Maximum block size.
    /// For any request larger than this won't be allocated with this allocator.
    pub max_block_size: u64,
}

/// Low-fragmentation allocator.
/// Suitable for any type of small allocations.
/// Have up to `block_size_granularity - 1` memory overhead.
/// Every freed block can be recycled independently.
/// Memory objects can be returned to the system if whole memory object become unused (not implemented yet).
#[derive(Debug)]
pub struct DynamicAllocator {
    /// Memory type that this allocator allocates.
    memory_type: u32,

    /// Memory properties of the memory type.
    memory_properties: MemoryPropertyFlags,

    /// Number of blocks per chunk.
    blocks_per_chunk: u32,

    /// All requests are rounded up to multiple of this value.
    block_size_granularity: u64,

    /// List of chunk lists.
    /// Each index corresponds to `block_size_granularity * index` size.
    sizes: Vec<Size>,
}

/// List of chunks
#[derive(Debug)]
struct Size {
    /// List of chunks.
    chunks: VecList<Chunk>,

    /// Total chunks count.
    total_chunks: u32,

    /// Bits per free blocks.
    blocks: BitSet,
}

impl DynamicAllocator {
    /// Get properties required by the allocator.
    pub fn properties_required() -> MemoryPropertyFlags {
        MemoryPropertyFlags::empty()
    }

    /// Maximum allocation size.
    pub fn max_allocation(&self) -> u64 {
        self.max_block_size()
    }

    /// Create new `ArenaAllocator`
    /// for `memory_type` with `memory_properties` specified,
    /// with `ArenaConfig` provided.
    pub fn new(
        memory_type: u32,
        memory_properties: MemoryPropertyFlags,
        mut config: DynamicConfig,
    ) -> Self {
        info!("Create new allocator: type: '{}', properties: '{}' config: '{:#?}'",
            memory_type, memory_properties, config);
        // This is hack to simplify implementation of chunk cleaning.
        config.blocks_per_chunk = std::mem::size_of::<usize>() as u32 * 8;

        assert_ne!(
            config.block_size_granularity, 0,
            "Allocation granularity can't be 0"
        );

        let max_chunk_size = config
            .max_block_size
            .checked_mul(config.blocks_per_chunk.into())
            .expect("Max chunk size must fit u64 to allocate it from Vulkan");
        if memory_properties.subset(MemoryPropertyFlags::HOST_VISIBLE) {
            assert!(
                fits_usize(max_chunk_size),
                "Max chunk size must fit usize for mapping"
            );
        }
        assert_eq!(
            config.max_block_size % config.block_size_granularity,
            0,
            "Max block size must be multiple of granularity"
        );

        let sizes = config.max_block_size / config.block_size_granularity;
        assert!(fits_usize(sizes), "Number of possible must fit usize");
        let sizes = sizes as usize;

        DynamicAllocator {
            memory_type,
            memory_properties,
            block_size_granularity: config.block_size_granularity,
            blocks_per_chunk: config.blocks_per_chunk,
            sizes: (0..sizes)
                .map(|_| Size {
                    chunks: VecList::new(),
                    blocks: BitSet::new(),
                    total_chunks: 0,
                }).collect(),
        }
    }

    /// Maximum block size.
    /// Any request bigger will result in panic.
    pub fn max_block_size(&self) -> u64 {
        (self.block_size_granularity * self.sizes.len() as u64)
    }

    fn max_chunks_per_size(&self) -> u32 {
        max_blocks_per_size() / self.blocks_per_chunk
    }

    /// Returns size index.
    fn size_index(&self, size: u64) -> usize {
        assert!(size <= self.max_block_size());
        ((size - 1) / self.block_size_granularity) as usize
    }

    /// Get block size for the size index.
    fn block_size(&self, index: usize) -> u64 {
        // Index must be acquired from `size_index` methods. Hence result is less than `max_block_size` and fits u64
        (self.block_size_granularity * (index as u64 + 1))
    }

    /// Allocate super-block to use as chunk memory.
    fn alloc_chunk(
        &mut self,
        device: &impl DeviceV1_0,
        size: u64,
    ) -> Result<(Chunk, u64), MemoryError> {
        // trace!("Allocate new chunk: size: {}", size);
        if size > self.max_block_size() {
            // Allocate from device.
            let (memory, mapping) = unsafe {
                // Valid memory type specified.
                let raw = device.allocate_memory(
                    &MemoryAllocateInfo {
                        memory_type_index: self.memory_type,
                        allocation_size: size,
                        ..Default::default()
                    },
                    None,
                )?;

                let mapping = if self
                    .memory_properties
                    .subset(MemoryPropertyFlags::HOST_VISIBLE)
                {
                    // trace!("Map new memory object");
                    match device.map_memory(raw, 0, size, MemoryMapFlags::empty()) {
                        Ok(mapping) => Some(NonNull::new_unchecked(mapping as *mut u8)),
                        Err(error) => {
                            device.free_memory(raw, None);
                            return Err(error.into());
                        }
                    }
                } else {
                    None
                };
                let memory = Memory::from_raw(raw, size, self.memory_properties);
                (memory, mapping)
            };
            Ok((Chunk::Dedicated(Box::new(memory), mapping), size))
        } else {
            // Allocate from larger chunks.
            let (dynamic_block, allocated) = self.alloc_from_chunk(device, size)?;
            Ok((Chunk::Dynamic(dynamic_block), allocated))
        }
    }

    /// Allocate super-block to use as chunk memory.
    #[warn(dead_code)]
    fn free_chunk(&mut self, device: &impl DeviceV1_0, chunk: Chunk) -> u64 {
        // trace!("Free chunk: {:#?}", chunk);
        match chunk {
            Chunk::Dedicated(boxed, _) => {
                let size = boxed.size();
                unsafe {
                    if self.memory_properties.subset(MemoryPropertyFlags::HOST_VISIBLE) {
                        // trace!("Unmap memory: {:#?}", boxed);
                        device.unmap_memory(boxed.raw());
                    }
                    device.free_memory(boxed.raw(), None);
                    boxed.dispose();
                }
                size
            }
            Chunk::Dynamic(dynamic_block) => self.free(device, dynamic_block),
        }
    }

    /// Allocate from chunk.
    fn alloc_from_chunk(
        &mut self,
        device: &impl DeviceV1_0,
        size: u64,
    ) -> Result<(DynamicBlock, u64), MemoryError> {
        // trace!("Allocate block. type: {}, size: {}", self.memory_type, size);
        let size_index = self.size_index(size);
        let (block_index, allocated) = match (&self.sizes[size_index].blocks).iter().next() {
            Some(block_index) => {
                self.sizes[size_index].blocks.remove(block_index);
                (block_index, 0)
            }
            None => {
                if self.sizes[self.size_index(size)].total_chunks == self.max_chunks_per_size() {
                    return Err(OutOfMemoryError::OutOfHostMemory.into());
                }
                let chunk_size = size * self.blocks_per_chunk as u64;
                let (chunk, allocated) = self.alloc_chunk(device, chunk_size)?;
                let chunk_index = self.sizes[size_index].chunks.push(chunk) as u32;
                self.sizes[size_index].total_chunks += 1;
                let block_index_start = chunk_index * self.blocks_per_chunk;
                let block_index_end = block_index_start + self.blocks_per_chunk;
                for block_index in block_index_start + 1..block_index_end {
                    let old = self.sizes[size_index].blocks.add(block_index);
                    debug_assert!(!old);
                }
                (block_index_start, allocated)
            }
        };

        let chunk_index = block_index / self.blocks_per_chunk;

        let ref chunk = self.sizes[size_index].chunks[chunk_index as usize];
        let chunk_range = chunk.range();
        let block_size = self.block_size(size_index);
        let block_offset =
            chunk_range.start + (block_index % self.blocks_per_chunk) as u64 * block_size;
        let block_range = block_offset..block_offset + block_size;

        Ok((
            DynamicBlock {
                range: block_range.clone(),
                memory: chunk.shared_memory(),
                index: block_index,
                ptr: chunk.ptr().map(|ptr| {
                    mapped_fitting_range(ptr, chunk.range(), block_range)
                        .expect("Block must be in sub-range of chunk")
                }),
                relevant: Relevant,
            },
            allocated,
        ))
    }

    /// Perform full cleanup of the memory allocated.
    pub fn dispose(self) {
        for size in self.sizes {
            assert_eq!(size.total_chunks, 0);
        }
    }
}

impl Allocator for DynamicAllocator {
    type Block = DynamicBlock;

    fn alloc(
        &mut self,
        device: &impl DeviceV1_0,
        size: u64,
        align: u64,
    ) -> Result<(DynamicBlock, u64), MemoryError> {
        use std::cmp::max;
        let size = max(size, align);

        assert!(size <= self.max_block_size());
        self.alloc_from_chunk(device, size)
    }

    fn free(&mut self, device: &impl DeviceV1_0, block: DynamicBlock) -> u64 {
        // trace!("Free block: {:#?}", block);
        let size_index = self.size_index(block.size());
        let block_index = block.index;
        block.dispose();

        let old = self.sizes[size_index].blocks.add(block_index);
        debug_assert!(!old);

        let chunk_index = block_index / self.blocks_per_chunk;
        let chunk_start = chunk_index * self.blocks_per_chunk;
        let chunk_end = chunk_start + self.blocks_per_chunk;

        if check_bit_range_set(&self.sizes[size_index].blocks, chunk_start..chunk_end) {
            for index in chunk_start..chunk_end {
                let old = self.sizes[size_index].blocks.remove(index);
                debug_assert!(old);
            }
            let chunk = self.sizes[size_index]
                .chunks
                .pop(chunk_index as usize)
                .expect("Chunk must exist");
            self.sizes[size_index].total_chunks -= 1;
            self.free_chunk(device, chunk)
        } else {
            0
        }
    }
}

/// Block allocated for chunk.
#[derive(Debug)]
enum Chunk {
    /// Allocated from device.
    Dedicated(Box<Memory>, Option<NonNull<u8>>),

    /// Allocated from chunk of bigger blocks.
    Dynamic(DynamicBlock),
}

unsafe impl Send for Chunk {}
unsafe impl Sync for Chunk {}

impl Chunk {
    fn shared_memory(&self) -> &Memory {
        match self {
            Chunk::Dedicated(boxed, _) => &*boxed,
            Chunk::Dynamic(chunk_block) => chunk_block.shared_memory(),
        }
    }

    fn range(&self) -> Range<u64> {
        match self {
            Chunk::Dedicated(boxed, _) => 0..boxed.size(),
            Chunk::Dynamic(chunk_block) => chunk_block.range(),
        }
    }

    fn ptr(&self) -> Option<NonNull<u8>> {
        match self {
            Chunk::Dedicated(_, ptr) => *ptr,
            Chunk::Dynamic(chunk_block) => chunk_block.ptr,
        }
    }
}

fn max_blocks_per_size() -> u32 {
    let value = (std::mem::size_of::<usize>() * 8).pow(4);
    assert!(fits_u32(value));
    value as u32
}

fn check_bit_range_set(bitset: &BitSet, range: Range<u32>) -> bool {
    debug_assert!(range.start <= range.end);
    use hibitset::BitSetLike;
    let layer_size = std::mem::size_of::<usize>() as u32 * 8;

    assert_eq!(
        range.start % layer_size,
        0,
        "Hack can be removed after this function works without this assert"
    );
    assert_eq!(
        range.end,
        range.start + layer_size,
        "Hack can be removed after this function works without this assert"
    );

    bitset.layer0((range.start / layer_size) as usize) == !0
}
