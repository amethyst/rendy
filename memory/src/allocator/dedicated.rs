
use std::{ops::Range, slice::from_raw_parts_mut, ptr::NonNull};
use block::Block;
use device::Device;
use error::*;
use map::*;
use memory::*;
use allocator::Allocator;
use util;

pub struct DedicatedAllocator {
    memory_type: u32,
    memory_properties: Properties,
}

pub struct DedicatedBlock<T> {
    memory: Memory<T>,
    mapping: Option<(NonNull<u8>, Range<u64>)>,
}

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

impl<T> Block<T> for DedicatedBlock<T> {
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
        D: Device<T>,
    {
        let coherent = maybe_coherent(self.memory.host_coherent());

        if !self.memory.host_visible() {
            return Err(MappingError::HostInvisible);
        } else if !util::fits_in_usize(range.end - range.start) || range.end > self.memory.size() {
            return Err(MappingError::OutOfBounds);
        } else if let Some(mapping) = self.mapping.clone() {
            debug_assert!(self.memory.host_visible());

            // Already mapped.
            if util::sub_range(mapping.1.clone(), range.clone()) {
                return Ok(unsafe {
                    MappedRange {
                        ptr: NonNull::new_unchecked(mapping.0.as_ptr().add((range.start - mapping.1.start) as usize)),
                        memory: self.memory.raw(),
                        offset: range.start,
                        length: (range.end - range.start) as usize,
                        coherent,
                    }
                });
            } else {
                self.mapping = None;
            }
        }

        Ok(unsafe {
            let ptr = device.map(self.memory.raw(), range.start .. range.end)?;
            self.mapping = Some((ptr, range.clone()));

            MappedRange {
                ptr,
                memory: self.memory.raw(),
                offset: range.start,
                length: (range.end - range.start) as usize,
                coherent,
            }
        })
    }

    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<T>,
    {
        if let Some(mapping) = self.mapping.take() {
            unsafe {
                device.unmap(self.memory());
            }
        }
    }
}

impl<T> Allocator<T> for DedicatedAllocator {
    type Block = DedicatedBlock<T>;

    #[inline]
    fn alloc<D>(&mut self, device: &D, size: u64, _align: u64) -> Result<(DedicatedBlock<T>, u64), MemoryError>
    where
        D: Device<T>,
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
        D: Device<T>,
    {
        block.unmap(device, 0 .. u64::max_value());
        let size = block.memory.size();
        unsafe {
            device.free(block.memory.into_raw());
        }
        size
    }
}
