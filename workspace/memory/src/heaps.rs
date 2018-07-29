
use std::ops::Range;

use allocator::Allocator;
use block::Block;
use memory::*;
use usage::Usage;

struct MemoryHeap {
    size: u64,
    used: u64,
}

impl MemoryHeap {
    fn available(&self) -> u64 {
        self.size - self.used
    }
}

struct MemoryType<A> {
    heap_index: u32,
    properties: Properties,
    allocator: A,
}

/// Heaps available on particular physical device.
pub struct Heaps<A> {
    types: Vec<MemoryType<A>>,
    heaps: Vec<MemoryHeap>,
}

impl<A> Heaps<A> {
    /// This must be called with `Properties` fetched from physical device.
    pub unsafe fn new(types: impl IntoIterator<Item = (Properties, u32)>, heaps: impl IntoIterator<Item = u64>) -> Self
    where
        A: Default,
    {
        Heaps {
            heaps: heaps.into_iter()
                .map(|size| MemoryHeap {
                    size,
                    used: 0,
                }).collect(),
            types: types.into_iter()
                .map(|(properties, heap_index)| MemoryType {
                    heap_index,
                    properties,
                    allocator: A::default(),
                }).collect(),
        }
    }

    #[cfg(feature = "gfx-hal")]
    /// Fetch data necessary from `Backend::PhysicalDevice`
    pub unsafe fn from_physical_device<B: ::hal::Backend>(physical: &B::PhysicalDevice) -> Self
    where
        A: Default,
    {
        let memory_properties = ::hal::PhysicalDevice::memory_properties(physical);
        Self::new(
            memory_properties.memory_types.into_iter().map(|mt| (mt.properties.into(), mt.heap_index as u32)),
            memory_properties.memory_heaps,
        )
    }
}

impl<A> Heaps<A> {
    pub fn allocate_from<D, T>(&mut self, device: &D, memory_index: u32, size: u64, align: u64) -> Result<HeapsBlock<A::Block>, MemoryError>
    where
        D: Device<T>,
        A: Allocator<T>,
    {
        let ref mut memory_type = self.types[memory_index as usize];
        let ref mut memory_heap = self.heaps[memory_type.heap_index as usize];

        if memory_heap.available() < size {
            return Err(MemoryError::OutOfDeviceMemory);
        }

        let (block, allocated) = memory_type.allocator.alloc(device, size, align)?;
        memory_heap.used += allocated;

        Ok(HeapsBlock {
            block,
            memory_index,
        })
    }

    pub fn allocate_for<D, T, U>(&mut self, device: &D, mask: u64, usage: U, size: u64, align: u64) -> Result<HeapsBlock<A::Block>, MemoryError>
    where
        D: Device<T>,
        A: Allocator<T>,
        U: Usage,
    {
        let (memory_index, _, _) = self.types.iter()
            .enumerate()
            .filter(|(index, _)| (mask & (1u64 << index)) != 0)
            .filter(|(_, mt)| self.heaps[mt.heap_index as usize].available() > size + align)
            .filter_map(|(index, mt)| usage.key(mt.properties).map(move |key| (index, mt, key)))
            .max_by_key(|&(_, _, key)| key)
            .ok_or(MemoryError::OutOfDeviceMemory)?;

        self.allocate_from::<D, T>(device, memory_index as u32, size, align)
    }

    pub fn free<D, T>(&mut self, device: &D, block: HeapsBlock<A::Block>)
    where
        D: Device<T>,
        A: Allocator<T>,
    {
        let memory_index = block.memory_index;
        let ref mut memory_type = self.types[memory_index as usize];
        let ref mut memory_heap = self.heaps[memory_type.heap_index as usize];
        let freed = memory_type.allocator.free(device, block.block);
        memory_heap.used -= freed;
    }
}

pub struct HeapsBlock<T> {
    block: T,
    memory_index: u32,
}

impl<T, M> Block<T> for HeapsBlock<M>
where
    M: Block<T>,
{
    #[inline]
    fn properties(&self) -> Properties {
        self.block.properties()
    }

    #[inline]
    fn memory(&self) -> &T {
        self.block.memory()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        self.block.range()
    }

    fn map<D>(&mut self, device: &D, range: Range<u64>) -> Result<&mut [u8], MappingError>
    where
        D: Device<T>,
    {
        self.block.map(device, range)
    }

    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<T>,
    {
        self.block.unmap(device, range)
    }
}


