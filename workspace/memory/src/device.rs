
use std::ops::Range;
use hal;

use allocator::Allocator;
use block::Block;
use sub::SubAllocator;
use Memory;

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

        let block = heap.allocate(
            size,
            align,
            move |size| Ok(Memory {
                raw: hal::Device::allocate_memory(device, memory_type_id, size)?,
                size,
                properties: memory_type.properties,
            })
        )?;

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
    fn free(&mut self, device: &B::Device, block: Self::Block) {
        let memory_type_id = block.memory_type_id;
        let ref mut heap = self.memory_heaps[self.memory_types[memory_type_id.0].heap_index];
        heap.free(block.block, move |memory| hal::Device::free_memory(device, memory.raw))
    }
}

pub struct DeviceMemoryBlock<T> {
    block: T,
    memory_type_id: hal::adapter::MemoryTypeId,
}

impl<T, M> Block<M> for DeviceMemoryBlock<T>
where
    T: Block<M>,
{
    #[inline]
    fn properties(&self) -> hal::memory::Properties {
        self.block.properties()
    }

    #[inline]
    fn memory(&mut self) -> &mut M {
        self.block.memory()
    }

    #[inline]
    unsafe fn lock(&mut self) {
        self.block.lock()
    }

    #[inline]
    unsafe fn unlock(&mut self) {
        self.block.lock()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        self.block.range()
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

    fn allocate<T, F, E>(&mut self, size: u64, align: u64, mut external: F) -> Result<A::Block, E>
    where
        A: SubAllocator<T>,
        F: FnMut(u64) -> Result<Memory<T>, E>,
    {
        let ref mut used = self.used;
        self.sub.sub_allocate(size, align, move |size| {
            let memory = external(size)?;
            *used += size;
            Ok(memory)
        })
    }

    fn free<T, F>(&mut self, block: A::Block, mut external: F)
    where
        A: SubAllocator<T>,
        F: FnMut(Memory<T>),
    {
        let ref mut used = self.used;
        self.sub.free(block, move |memory| {
            *used -= memory.size();
            external(memory)
        })
    }
}


