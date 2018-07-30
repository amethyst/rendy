
use std::{ops::Range, slice::from_raw_parts_mut, ptr::NonNull};
use block::Block;
use memory::{Memory, MemoryError, MappingError, Device, Properties};
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

    fn map<D>(&mut self, device: &D, range: Range<u64>) -> Result<&mut [u8], MappingError>
    where
        D: Device<T>,
    {
        if !self.memory.host_visible() {
            return Err(MappingError::HostInvisible);
        } else if !util::fits_in_usize(range.end - range.start) || range.end > self.memory.size() {
            return Err(MappingError::OutOfBounds);
        } else if range.start == range.end {
            return Ok(&mut [])
        } else if let Some(mapping) = self.mapping.clone() {
            debug_assert!(self.memory.host_visible());

            // Already mapped.
            if util::sub_range(mapping.1.clone(), range.clone()) {
                // Mapping contains requested range.
                if !self.memory.host_coherent() {
                    unsafe {
                        device.invalidate(Some((self.memory(), range.start .. range.end)));
                    }
                }

                return unsafe { // This pointer was created by successfully mapping with range bounds checked above.
                    Ok(from_raw_parts_mut(mapping.0.as_ptr().add((range.start - mapping.1.start) as usize), (range.end - range.start) as usize))
                };
            } else {
                // Requested range is out of mapping bounds.
                if !self.memory.host_coherent() {
                    unsafe {
                        device.flush(Some((self.memory(), mapping.1.clone())));
                    }
                }
                self.mapping = None;
            }
        }

        unsafe { // This pointer was created by successfully mapping specified range.
            let ptr = device.map(self.memory.raw(), range.start .. range.end)?;
            if !self.memory.host_coherent() {
                device.invalidate(Some((self.memory(), range.start .. range.end)));
            }

            self.mapping = Some((ptr, range.clone()));
            Ok(from_raw_parts_mut(ptr.as_ptr(), (range.end - range.start) as usize))
        }
    }

    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<T>,
    {
        if let Some(mapping) = self.mapping.take() {
            unsafe {
                if !self.memory.host_coherent() {
                    // Clamp to mapped range.
                    let range = util::clamp_range(range, mapping.1.clone());
                    device.flush(Some((self.memory(), range.start .. range.end)));
                }
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
