//! Adapter for gfx-hal


use std::{borrow::Borrow, marker::PhantomData, ops::Range, ptr::NonNull};
use hal::{self, Device as HalDevice};

use device::Device;
use error::*;
use memory::*;

impl From<hal::device::OutOfMemory> for OutOfMemoryError {
    fn from(_: hal::device::OutOfMemory) -> OutOfMemoryError {
        OutOfMemoryError::OutOfDeviceMemory
    }
}

impl From<hal::device::OutOfMemory> for MappingError {
    fn from(_: hal::device::OutOfMemory) -> MappingError {
        OutOfMemoryError::OutOfDeviceMemory.into()
    }
}

impl From<hal::device::OutOfMemory> for AllocationError {
    fn from(_: hal::device::OutOfMemory) -> AllocationError {
        OutOfMemoryError::OutOfDeviceMemory.into()
    }
}

impl From<hal::device::OutOfMemory> for MemoryError {
    fn from(_: hal::device::OutOfMemory) -> MemoryError {
        OutOfMemoryError::OutOfDeviceMemory.into()
    }
}

impl From<hal::mapping::Error> for MappingError {
    fn from(error: hal::mapping::Error) -> MappingError {
        match error {
            hal::mapping::Error::InvalidAccess => MappingError::HostInvisible,
            hal::mapping::Error::OutOfBounds => MappingError::OutOfBounds,
            hal::mapping::Error::OutOfMemory => OutOfMemoryError::OutOfHostMemory.into(),
        }
    }
}

impl From<hal::mapping::Error> for MemoryError {
    fn from(error: hal::mapping::Error) -> MemoryError {
        match error {
            hal::mapping::Error::InvalidAccess => MappingError::HostInvisible.into(),
            hal::mapping::Error::OutOfBounds => MappingError::OutOfBounds.into(),
            hal::mapping::Error::OutOfMemory => OutOfMemoryError::OutOfHostMemory.into(),
        }
    }
}


impl From<hal::memory::Properties> for Properties {
    fn from(value: hal::memory::Properties) -> Self {
        let mut result = Properties::empty();
        if value.contains(hal::memory::Properties::DEVICE_LOCAL) {
            result |= Properties::DEVICE_LOCAL;
        }
        if value.contains(hal::memory::Properties::COHERENT) {
            result |= Properties::HOST_COHERENT;
        }
        if value.contains(hal::memory::Properties::CPU_CACHED) {
            result |= Properties::HOST_CACHED;
        }
        if value.contains(hal::memory::Properties::CPU_VISIBLE) {
            result |= Properties::HOST_VISIBLE;
        }
        if value.contains(hal::memory::Properties::LAZILY_ALLOCATED) {
            result |= Properties::LAZILY_ALLOCATED;
        }
        result
    }
}

// impl Into<hal::memory::Properties> for Properties {
//     fn into(self) -> hal::memory::Properties {
//         assert!(!self.protected(), "Protected flag is not supported by gfx-hal");
//         let mut result = hal::memory::Properties::empty();
//         if self.device_local() {
//             result |= hal::memory::Properties::DEVICE_LOCAL;
//         }
//         if self.host_coherent() {
//             result |= hal::memory::Properties::COHERENT;
//         }
//         if self.host_cached() {
//             result |= hal::memory::Properties::CPU_CACHED;
//         }
//         if self.host_visible() {
//             result |= hal::memory::Properties::CPU_VISIBLE;
//         }
//         if self.lazily_allocated() {
//             result |= hal::memory::Properties::LAZILY_ALLOCATED;
//         }
//         result
//     }
// }

impl<D, B> Device<B::Memory> for (D, PhantomData<B>)
where
    B: hal::Backend,
    D: Borrow<B::Device>,
{
    unsafe fn allocate(&self, index: u32, size: u64) -> Result<B::Memory, AllocationError> {
        Ok(self.0.borrow().allocate_memory(hal::MemoryTypeId(index as usize), size)?)
    }

    unsafe fn free(&self, memory: B::Memory) {
        self.0.borrow().free_memory(memory)
    }

    unsafe fn map(&self, memory: &B::Memory, range: Range<u64>) -> Result<NonNull<u8>, MappingError> {
        let ptr = self.0.borrow().map_memory(memory, range)?;
        debug_assert!(!ptr.is_null());
        Ok(NonNull::new_unchecked(ptr))
    }

    unsafe fn unmap(&self, memory: &B::Memory) {
        self.0.borrow().unmap_memory(memory)
    }

    unsafe fn invalidate<'a>(&self, regions: impl IntoIterator<Item = (&'a B::Memory, Range<u64>)>) -> Result<(), OutOfMemoryError> {
        self.0.borrow().invalidate_mapped_memory_ranges(regions);
        Ok(())
    }

    unsafe fn flush<'a>(&self, regions: impl IntoIterator<Item = (&'a B::Memory, Range<u64>)>) -> Result<(), OutOfMemoryError> {
        self.0.borrow().flush_mapped_memory_ranges(regions);
        Ok(())
    }
}
