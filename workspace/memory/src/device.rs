
use std::{fmt::Debug, ops::Range};
use hal;

use allocator::Allocator;
use block::{Block, MappingError};
use sub::SubAllocator;
use Memory;
use usage::Usage;

pub struct DeviceMemory<A> {
    memory_types: Vec<hal::adapter::MemoryType>,
    memory_heaps: Vec<MemoryHeap<A>>,
}

impl<A> DeviceMemory<A> {
    /// Create device memory.
    pub unsafe fn new(device_properties: hal::adapter::MemoryProperties) -> Self
    where
        A: Default,
    {
        DeviceMemory {
            memory_heaps: device_properties.memory_heaps.into_iter().map(|size| {
                MemoryHeap {
                    size,
                    used: 0,
                    sub: A::default(),
                }
            }).collect(),
            memory_types: device_properties.memory_types,
        }
    }

    /// This must be called with `MemoryProperties` fetched from `PhysicalMemory`
    pub unsafe fn from_physical_device<B: hal::Backend>(physical: &B::PhysicalDevice)
    where
        A: Default,
    {
        Self::new(hal::PhysicalDevice::memory_properties(physical));
    }
}

impl<B, A> Allocator<B> for DeviceMemory<A>
where
    B: hal::Backend,
    A: SubAllocator<B::Memory>,
{
    type Block = DeviceMemoryBlock<A::Block>;

    fn allocate_from(&mut self, device: &B::Device, memory_type_id: hal::adapter::MemoryTypeId, size: u64, align: u64) -> Result<Self::Block, hal::device::OutOfMemory> {
        let ref memory_type = self.memory_types[memory_type_id.0];
        let ref mut heap = self.memory_heaps[memory_type.heap_index];

        let ref mut used = heap.used;
        let block = heap.sub.sub_allocate::<B, _, _>(device, size, align, move |size| {
            let memory = Memory {
                raw: hal::Device::allocate_memory(device, memory_type_id, size)?,
                size,
                properties: memory_type.properties,
            };
            *used += size;
            Ok(memory)
        })?;

        Ok(DeviceMemoryBlock {
            block,
            memory_type_id: memory_type_id,
        })
    }

    fn allocate_with(&mut self, device: &B::Device, mask: u64, properties: hal::memory::Properties, size: u64, align: u64) -> Result<Self::Block, hal::device::OutOfMemory> {
        use hal::memory::Properties;

        let (memory_type, _) = self.memory_types.iter()
            .enumerate()
            .filter(|(index, _)| (mask & (1u64 << index)) != 0)
            .filter(|(_, mt)| mt.properties.contains(properties))
            .filter(|(_, mt)| self.memory_heaps[mt.heap_index].available() > size + align)
            .max_by_key(|(_, mt)| {
                (
                    !(mt.properties ^ properties).contains(Properties::DEVICE_LOCAL),
                    !(mt.properties ^ properties).contains(Properties::CPU_VISIBLE),
                    !(mt.properties ^ properties).contains(Properties::LAZILY_ALLOCATED),
                    !(mt.properties ^ properties).contains(Properties::CPU_CACHED),
                    !(mt.properties ^ properties).contains(Properties::COHERENT),
                    self.memory_heaps[mt.heap_index].available(),
                )
            })
            .ok_or(hal::device::OutOfMemory)?;

        Allocator::<B>::allocate_from(self, device, hal::adapter::MemoryTypeId(memory_type), size, align)
    }

    fn allocate_for<U: Usage>(&mut self, device: &B::Device, mask: u64, usage: U, size: u64, align: u64) -> Result<Self::Block, hal::device::OutOfMemory> {
        let (memory_type, _, _) = self.memory_types.iter()
            .enumerate()
            .filter(|(index, _)| (mask & (1u64 << index)) != 0)
            .filter(|(_, mt)| self.memory_heaps[mt.heap_index].available() > size + align)
            .filter_map(|(index, mt)| usage.key(mt.properties).map(move |key| (index, mt, key)))
            .max_by_key(|&(_, _, key)| key)
            .ok_or(hal::device::OutOfMemory)?;

        Allocator::<B>::allocate_from(self, device, hal::adapter::MemoryTypeId(memory_type), size, align)
    }

    fn free(&mut self, device: &B::Device, block: Self::Block) {
        let memory_type_id = block.memory_type_id;
        let ref mut heap = self.memory_heaps[self.memory_types[memory_type_id.0].heap_index];
        let ref mut used = heap.used;
        heap.sub.free::<B, _>(device, block.block, move |memory| {
            *used -= memory.size();
            hal::Device::free_memory(device, memory.raw)
        })
    }
}

pub struct DeviceMemoryBlock<T> {
    block: T,
    memory_type_id: hal::adapter::MemoryTypeId,
}

impl<T, M> Block<T> for DeviceMemoryBlock<M>
where
    T: Debug + Send + Sync + 'static,
    M: Block<T>,
{
    #[inline]
    fn properties(&self) -> hal::memory::Properties {
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

    fn map<B>(&mut self, device: &B::Device, range: Range<u64>) -> Result<&mut [u8], MappingError>
    where
        T: Send + Sync + Debug + 'static,
        B: hal::Backend<Memory = T>,
    {
        self.block.map::<B>(device, range)
    }

    fn unmap<B>(&mut self, device: &B::Device, range: Range<u64>)
    where
        T: Send + Sync + Debug + 'static,
        B: hal::Backend<Memory = T>,
    {
        self.block.unmap::<B>(device, range)
    }
}

/// Memory heap.
struct MemoryHeap<A> {
    size: u64,
    used: u64,
    sub: A,
}

impl<A> MemoryHeap<A> {
    fn available(&self) -> u64 {
        self.size - self.used
    }
}


