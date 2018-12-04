use std::ops::Range;

use crate::{
    allocator::*,
    block::Block,
    mapping::*,
    usage::MemoryUsage,
    util::*,
};

/// Possible errors returned by `Heaps`.
#[allow(missing_copy_implementations)]
#[derive(Debug, failure::Fail)]
pub enum HeapsError {
    /// Memory allocation failure.
    #[fail(display = "{}", _0)]
    AllocationError(gfx_hal::device::AllocationError),

    /// No memory types among required for resource with requested properties was found.
    #[fail(display = "Memory type among ({}) with properties ({:?}) not found", _0, _1)]
    NoSuitableMemory(u32, gfx_hal::memory::Properties),
}

impl From<gfx_hal::device::AllocationError> for HeapsError {
    fn from(error: gfx_hal::device::AllocationError) -> Self {
        HeapsError::AllocationError(error)
    }
}

impl From<gfx_hal::device::OutOfMemory> for HeapsError {
    fn from(error: gfx_hal::device::OutOfMemory) -> Self {
        HeapsError::AllocationError(error.into())
    }
}

/// Config for `Heaps` allocator.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeapsConfig {
    /// Config for linear sub-allocator.
    pub linear: Option<LinearConfig>,

    /// Config for dynamic sub-allocator.
    pub dynamic: Option<DynamicConfig>,
}

/// Heaps available on particular physical device.
#[derive(Debug)]
pub struct Heaps<B: gfx_hal::Backend> {
    types: Vec<MemoryType<B>>,
    heaps: Vec<MemoryHeap>,
}

impl<B> Heaps<B>
where
    B: gfx_hal::Backend,
{
    /// This must be called with `gfx_hal::memory::Properties` fetched from physical device.
    pub unsafe fn new<P, H>(types: P, heaps: H) -> Self
    where
        P: IntoIterator<Item = (gfx_hal::memory::Properties, u32, HeapsConfig)>,
        H: IntoIterator<Item = u64>,
    {
        let heaps = heaps
            .into_iter()
            .map(|size| MemoryHeap::new(size))
            .collect::<Vec<_>>();
        Heaps {
            types: types
                .into_iter()
                .enumerate()
                .map(|(index, (properties, heap_index, config))| {
                    assert!(
                        fits_u32(index),
                        "Number of memory types must fit in u32 limit"
                    );
                    assert!(
                        fits_usize(heap_index),
                        "Number of memory types must fit in u32 limit"
                    );
                    let memory_type = gfx_hal::MemoryTypeId(index);
                    let heap_index = heap_index as usize;
                    assert!(heap_index < heaps.len());
                    MemoryType::new(memory_type, heap_index, properties, config)
                }).collect(),
            heaps,
        }
    }

    /// Allocate memory block
    /// from one of memory types specified by `mask`,
    /// for intended `usage`,
    /// with `size`
    /// and `align` requirements.
    pub fn allocate(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        mask: u32,
        usage: impl MemoryUsage,
        size: u64,
        align: u64,
    ) -> Result<MemoryBlock<B>, HeapsError> {
        debug_assert!(fits_u32(self.types.len()));

        let (memory_index, _, _) = {
            let suitable_types = self
                .types
                .iter()
                .enumerate()
                .filter(|(index, _)| (mask & (1u32 << index)) != 0)
                .filter_map(|(index, mt)| {
                    if mt.properties.contains(usage.properties_required()) {
                        let fitness = usage.memory_fitness(mt.properties);
                        Some((index, mt, fitness))
                    } else {
                        None
                    }
                }).collect::<smallvec::SmallVec<[_; 64]>>();

            if suitable_types.is_empty() {
                return Err(HeapsError::NoSuitableMemory(mask, usage.properties_required()));
            }

            suitable_types
                .into_iter()
                .filter(|(_, mt, _)| self.heaps[mt.heap_index].available() > size + align)
                .max_by_key(|&(_, _, fitness)| fitness)
                .ok_or_else(|| {
                    log::error!("All suitable heaps are exhausted. {:#?}", self);
                    gfx_hal::device::OutOfMemory::OutOfDeviceMemory
                })?
        };

        self.allocate_from(device, memory_index as u32, usage, size, align)
    }

    /// Allocate memory block
    /// from `memory_index` specified,
    /// for intended `usage`,
    /// with `size`
    /// and `align` requirements.
    fn allocate_from(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        memory_index: u32,
        usage: impl MemoryUsage,
        size: u64,
        align: u64,
    ) -> Result<MemoryBlock<B>, HeapsError> {
        // trace!("Alloc block: type '{}', usage '{:#?}', size: '{}', align: '{}'", memory_index, usage.value(), size, align);
        assert!(fits_usize(memory_index));

        let ref mut memory_type = self.types[memory_index as usize];
        let ref mut memory_heap = self.heaps[memory_type.heap_index];

        if memory_heap.available() < size {
            return Err(gfx_hal::device::OutOfMemory::OutOfDeviceMemory.into());
        }

        let (block, allocated) = memory_type.alloc(device, usage, size, align)?;
        memory_heap.used += allocated;

        Ok(MemoryBlock {
            block,
            memory_index,
        })
    }

    /// Free memory block.
    ///
    /// Memory block must be allocated from this heap.
    pub fn free(&mut self, device: &impl gfx_hal::Device<B>, block: MemoryBlock<B>) {
        // trace!("Free block '{:#?}'", block);
        let memory_index = block.memory_index;
        debug_assert!(fits_usize(memory_index));

        let ref mut memory_type = self.types[memory_index as usize];
        let ref mut memory_heap = self.heaps[memory_type.heap_index];
        let freed = memory_type.free(device, block.block);
        memory_heap.used -= freed;
    }

    /// Dispose of allocator.
    /// Cleanup allocators before dropping.
    /// Will panic if memory instances are left allocated.
    pub fn dispose(self, device: &impl gfx_hal::Device<B>) {
        for mt in self.types {
            mt.dispose(device)
        }
    }
}

/// Memory block allocated from `Heaps`.
#[derive(Debug)]
pub struct MemoryBlock<B: gfx_hal::Backend> {
    block: BlockFlavor<B>,
    memory_index: u32,
}

impl<B> MemoryBlock<B>
where
    B: gfx_hal::Backend
{
    /// Get memory type id.
    pub fn memory_type(&self) -> u32 {
        self.memory_index
    }
}

#[derive(Debug)]
enum BlockFlavor<B: gfx_hal::Backend> {
    Dedicated(DedicatedBlock<B>),
    Linear(LinearBlock<B>),
    Dynamic(DynamicBlock<B>),
    // Chunk(ChunkBlock<B>),
}

macro_rules! any_block {
    ($self:ident. $block:ident => $expr:expr) => {{
        use self::BlockFlavor::*;
        match $self.$block {
            Dedicated($block) => $expr,
            Linear($block) => $expr,
            Dynamic($block) => $expr,
            // Chunk($block) => $expr,
        }
    }};
    (& $self:ident. $block:ident => $expr:expr) => {{
        use self::BlockFlavor::*;
        match &$self.$block {
            Dedicated($block) => $expr,
            Linear($block) => $expr,
            Dynamic($block) => $expr,
            // Chunk($block) => $expr,
        }
    }};
    (&mut $self:ident. $block:ident => $expr:expr) => {{
        use self::BlockFlavor::*;
        match &mut $self.$block {
            Dedicated($block) => $expr,
            Linear($block) => $expr,
            Dynamic($block) => $expr,
            // Chunk($block) => $expr,
        }
    }};
}

impl<B> Block<B> for MemoryBlock<B>
where
    B: gfx_hal::Backend,
{
    #[inline]
    fn properties(&self) -> gfx_hal::memory::Properties {
        any_block!(&self.block => block.properties())
    }

    #[inline]
    fn memory(&self) -> &B::Memory {
        any_block!(&self.block => block.memory())
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        any_block!(&self.block => block.range())
    }

    fn map<'a>(
        &'a mut self,
        device: &impl gfx_hal::Device<B>,
        range: Range<u64>,
    ) -> Result<MappedRange<'a, B>, gfx_hal::mapping::Error> {
        any_block!(&mut self.block => block.map(device, range))
    }

    fn unmap(&mut self, device: &impl gfx_hal::Device<B>) {
        any_block!(&mut self.block => block.unmap(device))
    }
}

#[derive(Debug)]
struct MemoryHeap {
    size: u64,
    used: u64,
}

impl MemoryHeap {
    fn new(size: u64) -> Self {
        MemoryHeap { size, used: 0 }
    }

    fn available(&self) -> u64 {
        self.size - self.used
    }
}

#[derive(Debug)]
struct MemoryType<B: gfx_hal::Backend> {
    heap_index: usize,
    properties: gfx_hal::memory::Properties,
    dedicated: DedicatedAllocator,
    linear: Option<LinearAllocator<B>>,
    dynamic: Option<DynamicAllocator<B>>,
    // chunk: Option<ChunkAllocator>,
}

impl<B> MemoryType<B>
where
    B: gfx_hal::Backend
{
    fn new(
        memory_type: gfx_hal::MemoryTypeId,
        heap_index: usize,
        properties: gfx_hal::memory::Properties,
        config: HeapsConfig,
    ) -> Self {
        MemoryType {
            properties,
            heap_index,
            dedicated: DedicatedAllocator::new(memory_type, properties),
            linear: if properties.contains(gfx_hal::memory::Properties::CPU_VISIBLE) {
                config
                    .linear
                    .map(|config| LinearAllocator::new(memory_type, properties, config))
            } else {
                None
            },
            dynamic: config
                .dynamic
                .map(|config| DynamicAllocator::new(memory_type, properties, config)),
        }
    }

    fn alloc(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        usage: impl MemoryUsage,
        size: u64,
        align: u64,
    ) -> Result<(BlockFlavor<B>, u64), gfx_hal::device::AllocationError> {
        match (self.dynamic.as_mut(), self.linear.as_mut()) {
            (Some(dynamic), Some(linear)) => {
                if dynamic.max_allocation() >= size && usage.allocator_fitness(Kind::Dynamic) > usage.allocator_fitness(Kind::Linear) {
                    dynamic.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Dynamic(block), size))
                } else if linear.max_allocation() >= size && usage.allocator_fitness(Kind::Linear) > 0 {
                    linear.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Linear(block), size))
                } else {
                    self.dedicated.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Dedicated(block), size))
                }
            },
            (Some(dynamic), None) => {
                if dynamic.max_allocation() >= size && usage.allocator_fitness(Kind::Dynamic) > 0 {
                    dynamic.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Dynamic(block), size))
                } else {
                    self.dedicated.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Dedicated(block), size))
                }
            },
            (None, Some(linear)) => {
                if linear.max_allocation() >= size && usage.allocator_fitness(Kind::Linear) > 0 {
                    linear.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Linear(block), size))
                } else {
                    self.dedicated.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Dedicated(block), size))
                }
            },
            (None, None) => {
                self.dedicated.alloc(device, size, align).map(|(block, size)| (BlockFlavor::Dedicated(block), size))
            }
        }
    }

    fn free(&mut self, device: &impl gfx_hal::Device<B>, block: BlockFlavor<B>) -> u64 {
        match block {
            BlockFlavor::Dedicated(block) => self.dedicated.free(device, block),
            BlockFlavor::Linear(block) => self.linear.as_mut().unwrap().free(device, block),
            BlockFlavor::Dynamic(block) => self.dynamic.as_mut().unwrap().free(device, block),
        }
    }

    fn dispose(self, device: &impl gfx_hal::Device<B>) {
        if let Some(linear) = self.linear {
            linear.dispose(device);
        }
        if let Some(dynamic) = self.dynamic {
            dynamic.dispose();
        }
    }
}
