
use std::{fmt::Debug, ops::Range, slice::from_raw_parts_mut, ptr::NonNull};
use hal;
use block::{Block, MappingError};
use memory::Memory;
use sub::SubAllocator;

pub struct DedicatedAllocator;
pub struct DedicatedBlock<T> {
    memory: Memory<T>,
    mapping: Option<(NonNull<u8>, Range<u64>)>,
}

impl<T> Block<T> for DedicatedBlock<T>
where
    T: Debug + Send + Sync + 'static,
{
    #[inline]
    fn properties(&self) -> hal::memory::Properties {
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

    fn map<B>(&mut self, device: &B::Device, range: Range<u64>) -> Result<&mut [u8], MappingError>
    where
        B: hal::Backend<Memory = T>,
    {
        if !self.memory.cpu_visible() {
            return Err(MappingError::HostInvisible);
        } else if !super::fits_in_usize(range.end - range.start) || range.end > self.memory.size() {
            return Err(MappingError::OutOfBounds);
        } else if range.start == range.end {
            return Ok(&mut [])
        } else if let Some(mapping) = self.mapping.clone() {
            debug_assert!(self.memory.cpu_visible());

            // Already mapped.
            if super::sub_range(mapping.1.clone(), range.clone()) {
                // Mapping contains requested range.
                if !self.memory.coherent() {
                    hal::Device::invalidate_mapped_memory_ranges(device, Some((self.memory(), range.start .. range.end)));
                }

                return unsafe { // This pointer was created by successfully mapping with range bounds checked above.
                    Ok(from_raw_parts_mut(mapping.0.as_ptr().add((range.start - mapping.1.start) as usize), (range.end - range.start) as usize))
                };
            } else {
                // Requested range is out of mapping bounds.
                if !self.memory.coherent() {
                    hal::Device::flush_mapped_memory_ranges(device, Some((self.memory(), mapping.1.clone())));
                }
                self.mapping = None;
            }
        }

        match hal::Device::map_memory(device, self.memory.raw(), range.start .. range.end) {
            Ok(ptr) => {
                debug_assert!(!ptr.is_null());
                if !self.memory.coherent() {
                    hal::Device::invalidate_mapped_memory_ranges(device, Some((self.memory(), range.start .. range.end)));
                }

                unsafe { // This pointer was created by successfully mapping specified range.
                    self.mapping = Some((NonNull::new_unchecked(ptr), range.clone()));
                    Ok(from_raw_parts_mut(ptr, (range.end - range.start) as usize))
                }
            }
            Err(hal::mapping::Error::InvalidAccess) => unreachable!("Memory properties checked"),
            Err(hal::mapping::Error::OutOfBounds) => unreachable!("range checked"),
            Err(hal::mapping::Error::OutOfMemory) => Err(MappingError::OutOfHostMemory),
        }
    }

    fn unmap<B>(&mut self, device: &B::Device, range: Range<u64>)
    where
        B: hal::Backend<Memory = T>,
    {
        if let Some(mapping) = self.mapping.take() {
            if !self.memory.coherent() {
                // Clamp to mapped range.
                let range = super::clamp_range(range, mapping.1.clone());
                hal::Device::flush_mapped_memory_ranges(device, Some((self.memory(), range.start .. range.end)));
            }
            hal::Device::unmap_memory(device, self.memory());
        }
    }
}

impl<T> SubAllocator<T> for DedicatedAllocator
where
    T: Debug + Send + Sync + 'static,
{
    type Block = DedicatedBlock<T>;

    #[inline]
    fn sub_allocate<B, F, E>(&mut self, _: &B::Device, size: u64, _align: u64, external: F) -> Result<DedicatedBlock<T>, E>
    where
        B: hal::Backend,
        F: FnOnce(u64) -> Result<Memory<T>, E>,
    {
        Ok(DedicatedBlock {
            memory: external(size)?,
            mapping: None,
        })
    }

    #[inline]
    fn free<B, F>(&mut self, _: &B::Device, block: Self::Block, external: F)
    where
        B: hal::Backend,
        F: FnOnce(Memory<T>),
    {
        external(block.memory)
    }
}
