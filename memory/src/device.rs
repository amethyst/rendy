
use std::{ops::Range, ptr::NonNull, fmt::Debug, any::Any};
use error::*;

/// Trait for memory allocation and mapping.
pub trait Device {
    type Memory: Any;

    /// Allocate memory object.
    /// 
    /// # Parameters
    /// `size`  - size of the memory object to allocate.
    /// `index` - memory type index.
    unsafe fn allocate(&self, index: u32, size: u64) -> Result<Self::Memory, AllocationError>;

    /// Free memory object.
    unsafe fn free(&self, memory: Self::Memory);

    /// Map memory range.
    /// Only one range for the given memory object can be mapped.
    unsafe fn map(&self, memory: &Self::Memory, range: Range<u64>) -> Result<NonNull<u8>, MappingError>;

    /// Unmap memory.
    unsafe fn unmap(&self, memory: &Self::Memory);

    /// Invalidate mapped regions guaranteeing that device writes to the memory,
    /// which have been made visible to the host-write and host-read access types, are made visible to the host
    unsafe fn invalidate<'a>(&self, regions: impl IntoIterator<Item = (&'a Self::Memory, Range<u64>)>) -> Result<(), OutOfMemoryError>;

    /// Flush mapped regions guaranteeing that host writes to the memory can be made available to device access
    unsafe fn flush<'a>(&self, regions: impl IntoIterator<Item = (&'a Self::Memory, Range<u64>)>) -> Result<(), OutOfMemoryError>;
}
