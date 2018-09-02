
use std::{ops::Range, ptr::NonNull, marker::PhantomData};
use block::Block;
use device::Device;
use error::*;
use mapping::{mapped_fitting_range, MappedRange};
use memory::*;
use allocator::Allocator;
use util::*;

#[derive(Debug)]
pub struct DedicatedBlock<T> {
    memory: Memory<T>,
    mapping: Option<(NonNull<u8>, Range<u64>)>,
}

unsafe impl<T: Send> Send for DedicatedBlock<T> {}
unsafe impl<T: Sync> Sync for DedicatedBlock<T> {}

impl<T> DedicatedBlock<T> {
    /// Get inner memory.
    /// Panics if mapped.
    pub fn unwrap_memory(self) -> Memory<T> {
        assert!(self.mapping.is_none());
        self.memory
    }

    /// Make unmapped block.
    pub fn from_memory(memory: Memory<T>) -> Self {
        DedicatedBlock {
            memory,
            mapping: None,
        }
    }
}

impl<T: 'static> Block for DedicatedBlock<T> {

    type Memory = T;

    #[inline]
    fn properties(&self) -> Properties {
        self.memory.properties()
    }

    #[inline]
    fn memory(&self) -> &T {
        self.memory.raw()
    }

    #[inline]
    fn range(&self) -> Range<u64> {
        0 .. self.memory.size()
    }

    fn map<'a, D>(&'a mut self, device: &D, range: Range<u64>) -> Result<MappedRange<'a, T>, MappingError>
    where
        D: Device<Memory = T>,
    {
        assert!(range.start <= range.end, "Memory mapping region must have valid size");

        unsafe {
            if let Some(ptr) = self.mapping
                .clone()
                .and_then(|mapping| mapped_fitting_range(mapping.0, mapping.1, range.clone()))
            {
                Ok(MappedRange::from_raw(&self.memory, ptr, range))
            } else {
                if self.mapping.take().is_some() {
                    device.unmap(&self.memory.raw());
                }
                let mapping = MappedRange::new(&self.memory, device, range.clone())?;
                self.mapping = Some((mapping.ptr(), mapping.range()));
                Ok(mapping)
            }
        }
    }

    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<Memory = T>,
    {
        if let Some(mapping) = self.mapping.take() {
            unsafe {
                device.unmap(self.memory());
            }
        }
    }
}

pub struct DedicatedAllocator<T> {
    memory_type: u32,
    memory_properties: Properties,
    pd: PhantomData<T>,
}

impl<T> DedicatedAllocator<T> {

    pub fn properties_required() -> Properties {
        Properties::empty()
    }

    pub fn new(
        memory_type: u32,
        memory_properties: Properties,
    ) -> Self {
        DedicatedAllocator {
            memory_type,
            memory_properties,
            pd: PhantomData,
        }
    }
}

impl<T: 'static> Allocator for DedicatedAllocator<T> {

    type Memory = T;
    type Block = DedicatedBlock<T>;

    #[inline]
    fn alloc<D>(&mut self, device: &D, size: u64, _align: u64) -> Result<(DedicatedBlock<T>, u64), MemoryError>
    where
        D: Device<Memory = T>,
    {
        let memory = unsafe {
            Memory::from_raw(device.allocate(self.memory_type, size)?, size, self.memory_properties)
        };
        Ok((DedicatedBlock {
            memory,
            mapping: None,
        }, size))
    }

    #[inline]
    fn free<D>(&mut self, device: &D, mut block: DedicatedBlock<T>) -> u64
    where
        D: Device<Memory = T>,
    {
        block.unmap(device, 0 .. u64::max_value());
        let size = block.memory.size();
        unsafe {
            device.free(block.memory.into_raw());
        }
        size
    }
}
