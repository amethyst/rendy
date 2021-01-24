mod heap;
mod memory_type;

use {
    self::{heap::MemoryHeap, memory_type::MemoryType},
    crate::{allocator::*, block::Block, mapping::*, usage::MemoryUsage, utilization::*},
    std::ops::Range,
};

/// Possible errors returned by `Heaps`.
#[allow(missing_copy_implementations)]
#[derive(Clone, Debug, PartialEq)]
pub enum HeapsError {
    /// Memory allocation failure.
    AllocationError(gfx_hal::device::AllocationError),
    /// No memory types among required for resource with requested properties was found.
    NoSuitableMemory(u32, gfx_hal::memory::Properties),
}

impl std::fmt::Display for HeapsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeapsError::AllocationError(e) => write!(f, "{:?}", e),
            HeapsError::NoSuitableMemory(e, e2) => write!(
                f,
                "Memory type among ({}) with properties ({:?}) not found",
                e, e2
            ),
        }
    }
}
impl std::error::Error for HeapsError {}

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub unsafe fn new<P, H>(types: P, heaps: H, non_coherent_atom_size: u64) -> Self
    where
        P: IntoIterator<Item = (gfx_hal::memory::Properties, u32, HeapsConfig)>,
        H: IntoIterator<Item = u64>,
    {
        Heaps {
            types: types
                .into_iter()
                .enumerate()
                .map(|(index, (properties, heap_index, config))| {
                    let memory_type = gfx_hal::MemoryTypeId(index);
                    let heap_index = heap_index as usize;
                    MemoryType::new(
                        memory_type,
                        heap_index,
                        properties,
                        config,
                        non_coherent_atom_size,
                    )
                })
                .collect(),
            heaps: heaps.into_iter().map(MemoryHeap::new).collect::<Vec<_>>(),
        }
    }

    /// Allocate memory block
    /// from one of memory types specified by `mask`,
    /// for intended `usage`,
    /// with `size`
    /// and `align` requirements.
    pub fn allocate(
        &mut self,
        device: &B::Device,
        mask: u32,
        usage: impl MemoryUsage,
        size: u64,
        align: u64,
    ) -> Result<MemoryBlock<B>, HeapsError> {
        let (memory_index, _, _) = {
            let suitable_types = self
                .types
                .iter()
                .enumerate()
                .filter(|(index, _)| (mask & (1u32 << index)) != 0)
                .filter_map(|(index, mt)| {
                    if mt.properties().contains(usage.properties_required()) {
                        let fitness = usage.memory_fitness(mt.properties());
                        Some((index, mt, fitness))
                    } else {
                        None
                    }
                })
                .collect::<smallvec::SmallVec<[_; 64]>>();

            if suitable_types.is_empty() {
                return Err(HeapsError::NoSuitableMemory(
                    mask,
                    usage.properties_required(),
                ));
            }

            suitable_types
                .into_iter()
                .filter(|(_, mt, _)| self.heaps[mt.heap_index()].available() > size + align)
                .max_by_key(|&(_, _, fitness)| fitness)
                .ok_or(gfx_hal::device::OutOfMemory::Device)?
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
        device: &B::Device,
        memory_index: u32,
        usage: impl MemoryUsage,
        size: u64,
        align: u64,
    ) -> Result<MemoryBlock<B>, HeapsError> {
        let memory_type = &mut self.types[memory_index as usize];
        let memory_heap = &mut self.heaps[memory_type.heap_index()];

        if memory_heap.available() < size {
            return Err(gfx_hal::device::OutOfMemory::Device.into());
        }

        let (block, allocated) = memory_type.alloc(device, usage, size, align)?;
        memory_heap.allocated(allocated, block.size());

        Ok(MemoryBlock {
            block,
            memory_index,
        })
    }

    /// Free memory block.
    ///
    /// Memory block must be allocated from this heap.
    pub fn free(&mut self, device: &B::Device, block: MemoryBlock<B>) {
        let memory_index = block.memory_index;
        let size = block.size();

        let memory_type = &mut self.types[memory_index as usize];
        let memory_heap = &mut self.heaps[memory_type.heap_index()];
        let freed = memory_type.free(device, block.block);
        memory_heap.freed(freed, size);
    }

    /// Dispose of allocator.
    /// Cleanup allocators before dropping.
    /// Will panic if memory instances are left allocated.
    pub fn dispose(self, device: &B::Device) {
        for mt in self.types {
            mt.dispose(device)
        }
    }

    /// Get memory utilization.
    pub fn utilization(&self) -> TotalMemoryUtilization {
        TotalMemoryUtilization {
            heaps: self.heaps.iter().map(MemoryHeap::utilization).collect(),
            types: self.types.iter().map(MemoryType::utilization).collect(),
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
    B: gfx_hal::Backend,
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

impl<B> BlockFlavor<B>
where
    B: gfx_hal::Backend,
{
    #[inline]
    fn size(&self) -> u64 {
        use self::BlockFlavor::*;
        match self {
            Dedicated(block) => block.size(),
            Linear(block) => block.size(),
            Dynamic(block) => block.size(),
            // Chunk(block) => block.size(),
        }
    }
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
        device: &B::Device,
        range: Range<u64>,
    ) -> Result<MappedRange<'a, B>, gfx_hal::device::MapError> {
        any_block!(&mut self.block => block.map(device, range))
    }

    fn unmap(&mut self, device: &B::Device) {
        any_block!(&mut self.block => block.unmap(device))
    }
}
