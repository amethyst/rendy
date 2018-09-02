
use std::ops::Range;

use allocator::{
    Allocator,
    dedicated::*,
    arena::*,
    dynamic::*,
    // chunk::*,
};

use block::Block;
use device::Device;
use error::*;
use mapping::*;
use memory::*;
use usage::{Usage, UsageValue};
use util::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeapsConfig {
    pub arena: Option<ArenaConfig>,
    pub dynamic: Option<DynamicConfig>,
    // chunk: Option<ChunkConfig>,
}

/// Heaps available on particular physical device.
pub struct Heaps<T> {
    types: Vec<MemoryType<T>>,
    heaps: Vec<MemoryHeap>,
}

impl<T: 'static> Heaps<T> {
    /// This must be called with `Properties` fetched from physical device.
    pub unsafe fn new<P, H>(types: P, heaps: H, config: HeapsConfig) -> Self
    where
        P: IntoIterator<Item = (Properties, u32)>,
        H: IntoIterator<Item = u64>,
    {        
        Heaps {
            heaps: heaps.into_iter()
                .map(|size| MemoryHeap::new(size))
                .collect(),
            types: types.into_iter()
                .enumerate()
                .map(|(index, (properties, heap_index))| {
                    debug_assert!(fits_u32(index), "Number of memory types must fit in u32 limit");
                    debug_assert!(fits_usize(heap_index), "Number of memory types must fit in u32 limit");
                    let memory_type = index as u32;
                    MemoryType::new(memory_type, heap_index as usize, properties, config)
                })
                .collect(),
        }
    }

    pub fn allocate<D, U>(&mut self, device: &D, mask: u32, usage: U, size: u64, align: u64) -> Result<MemoryBlock<T>, MemoryError>
    where
        D: Device<Memory = T>,
        U: Usage,
    {
        debug_assert!(fits_u32(self.types.len()));
        let (memory_index, _, _) = self.types.iter()
            .enumerate()
            .filter(|(index, _)| (mask & (1u32 << index)) != 0)
            .filter(|(_, mt)| self.heaps[mt.heap_index].available() > size + align)
            .filter_map(|(index, mt)| usage.memory_fitness(mt.properties).map(move |fitness| (index, mt, fitness)))
            .max_by_key(|&(_, _, fitness)| fitness)
            .ok_or(OutOfMemoryError::OutOfDeviceMemory)?;

        self.allocate_from::<D, U>(device, memory_index as u32, usage, size, align)
    }

    fn allocate_from<D, U>(&mut self, device: &D, memory_index: u32, usage: U, size: u64, align: u64) -> Result<MemoryBlock<T>, MemoryError>
    where
        D: Device<Memory = T>,
        U: Usage,
    {
        assert!(fits_usize(memory_index));

        let ref mut memory_type = self.types[memory_index as usize];
        let ref mut memory_heap = self.heaps[memory_type.heap_index];

        if memory_heap.available() < size {
            return Err(OutOfMemoryError::OutOfDeviceMemory.into());
        }

        let (block, allocated) = memory_type.alloc(device, usage, size, align)?;
        memory_heap.used += allocated;

        Ok(MemoryBlock {
            block,
            memory_index,
        })
    }

    pub fn free<D>(&mut self, device: &D, block: MemoryBlock<T>)
    where
        D: Device<Memory = T>,
    {
        let memory_index = block.memory_index;
        debug_assert!(fits_usize(memory_index));

        let ref mut memory_type = self.types[memory_index as usize];
        let ref mut memory_heap = self.heaps[memory_type.heap_index];
        let freed = memory_type.free(device, block.block);
        memory_heap.used -= freed;
    }
}

#[derive(Debug)]
pub struct MemoryBlock<T> {
    block: BlockFlavor<T>,
    memory_index: u32,
}

#[derive(Debug)]
enum BlockFlavor<T> {
    Dedicated(DedicatedBlock<T>),
    Arena(ArenaBlock<T>),
    Dynamic(DynamicBlock<T>),
    // Chunk(ChunkBlock<T>),
}


macro_rules! any_block {
    ($self:ident.$block:ident => $expr:expr) => {{
        use self::BlockFlavor::*;
        match $self.$block {
            Dedicated($block) => $expr,
            Arena($block) => $expr,
            Dynamic($block) => $expr,
            // Chunk($block) => $expr,
        }
    }};
    (&$self:ident.$block:ident => $expr:expr) => {{
        use self::BlockFlavor::*;
        match &$self.$block {
            Dedicated($block) => $expr,
            Arena($block) => $expr,
            Dynamic($block) => $expr,
            // Chunk($block) => $expr,
        }
    }};
    (&mut $self:ident.$block:ident => $expr:expr) => {{
        use self::BlockFlavor::*;
        match &mut $self.$block {
            Dedicated($block) => $expr,
            Arena($block) => $expr,
            Dynamic($block) => $expr,
            // Chunk($block) => $expr,
        }
    }};
}

impl<T: 'static> Block for MemoryBlock<T> {

    type Memory = T;

    #[inline]
    fn properties(&self) -> Properties {
        any_block!(&self.block => block.properties())
    }

    #[inline]
    fn memory(&self) -> &T {
        any_block!(&self.block => block.memory())
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        any_block!(&self.block => block.range())
    }

    fn map<'a, D>(&'a mut self, device: &D, range: Range<u64>) -> Result<MappedRange<'a, T>, MappingError>
    where
        D: Device<Memory = T>,
    {
        any_block!(&mut self.block => block.map(device, range))
    }

    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<Memory = T>,
    {
        any_block!(&mut self.block => block.unmap(device, range))
    }
}


struct MemoryHeap {
    size: u64,
    used: u64,
}

impl MemoryHeap {
    fn new(size: u64) -> Self {
        MemoryHeap {
            size,
            used: 0,
        }
    }

    fn available(&self) -> u64 {
        self.size - self.used
    }
}

struct MemoryType<T> {
    heap_index: usize,
    properties: Properties,
    dedicated: DedicatedAllocator<T>,
    arena: Option<ArenaAllocator<T>>,
    dynamic: Option<DynamicAllocator<T>>,
    // chunk: Option<ChunkAllocator<T>>,
}

impl<T: 'static> MemoryType<T> {
    fn new(memory_type: u32, heap_index: usize, properties: Properties, config: HeapsConfig) -> Self {
        MemoryType {
            properties,
            heap_index,
            dedicated: DedicatedAllocator::new(memory_type, properties),
            arena: if properties.contains(ArenaAllocator::<T>::properties_required()) {
                config.arena.map(|config| ArenaAllocator::new(memory_type, properties, config))
            } else {
                None
            },
            dynamic: if properties.contains(DynamicAllocator::<T>::properties_required()) {
                config.dynamic.map(|config| DynamicAllocator::new(memory_type, properties, config))
            } else {
                None
            },
            // chunk: if properties.contains(ChunkAllocator::<T>::properties_required()) {
            //     config.chunk.map(|config| ChunkAllocator::new(memory_type, properties, config))
            // } else {
            //     None
            // },
        }
    }

    fn alloc<D, U>(&mut self, device: &D, usage: U, size: u64, align: u64) -> Result<(BlockFlavor<T>, u64), MemoryError>
    where
        D: Device<Memory = T>,
        U: Usage,
    {
        match (usage.value(), self.arena.as_mut(), self.dynamic.as_mut()) {
            (UsageValue::Upload, Some(ref mut arena), _) | (UsageValue::Download, Some(ref mut arena), _) if size <= arena.max_allocation() => {
                arena.alloc(device, size, align).map(|(block, allocated)| (BlockFlavor::Arena(block), allocated))
            },
            (UsageValue::Dynamic, _, Some(ref mut dynamic)) if size <= dynamic.max_allocation() => {
                dynamic.alloc(device, size, align).map(|(block, allocated)| (BlockFlavor::Dynamic(block), allocated))
            },
            (UsageValue::Data, _, Some(ref mut dynamic)) if size <= dynamic.max_allocation() => {
                dynamic.alloc(device, size, align).map(|(block, allocated)| (BlockFlavor::Dynamic(block), allocated))
            },
            _ => self.dedicated.alloc(device, size, align).map(|(block, allocated)| (BlockFlavor::Dedicated(block), allocated)),
        }
    }

    fn free<D>(&mut self, device: &D, block: BlockFlavor<T>) -> u64
    where
        D: Device<Memory = T>,
    {
        match block {
            BlockFlavor::Dedicated(block) => self.dedicated.free(device, block),
            BlockFlavor::Arena(block) => self.arena.as_mut().unwrap().free(device, block),
            BlockFlavor::Dynamic(block) => self.dynamic.as_mut().unwrap().free(device, block),
            // BlockFlavor::Chunk(block) => self.chunk.free(device, block),
        }
    }
}
