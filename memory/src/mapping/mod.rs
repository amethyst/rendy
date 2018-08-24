
mod range;
mod write;

use std::{
    fmt::Debug,
    ops::Range,
    ptr::NonNull,
};

use device::Device;
use error::{MappingError, MemoryError};
use memory::Memory;
use util::fits_usize;

pub use self::range::{mapped_fitting_range, mapped_slice, mapped_slice_mut, mapped_sub_range};
pub use self::write::Write;
use self::write::{WriteFlush, WriteCoherent};

/// Non-coherent marker.
pub struct NonCoherent;

/// Coherent marker.
pub struct Coherent;

/// Value that contains either coherent marker or non-coherent marker.
pub struct MaybeCoherent(bool);

#[derive(Debug)]
pub struct MappedRange<'a, T: 'static, C = MaybeCoherent> {
    /// Memory object that is mapped.
    memory: &'a T,

    /// Pointer to range mapped memory.
    ptr: NonNull<u8>,

    /// Range of mapped memory.
    range: Range<u64>,

    /// Coherency marker
    coherent: C,
}

impl<'a, T: 'static> MappedRange<'a, T, MaybeCoherent> {
    /// Map range of memory.
    ///
    /// # Safety
    ///
    /// Only one range for the given memory object can be mapped.
    /// Memory object must be not mapped.
    /// Memory object must be created with device specified.
    pub unsafe fn new<D>(memory: &'a Memory<T>, device: &D, range: Range<u64>) -> Result<Self, MappingError>
    where
        D: Device<Memory = T>,
    {
        assert!(range.start <= range.end, "Memory mapping region must have valid size");
        assert!(fits_usize(range.end - range.start), "Range length must fit in usize");
        assert!(memory.host_visible());

        let ptr = device.map(memory.raw(), range.clone())?;
        assert!(
            (ptr.as_ptr() as usize).wrapping_neg() <= (range.end - range.start) as usize,
            "Resulting pointer value + range length must fit in usize",
        );

        Ok(Self::from_raw(memory, ptr, range))
    }

    /// Construct mapped range from raw mapping
    pub unsafe fn from_raw(memory: &'a Memory<T>, ptr: NonNull<u8>, range: Range<u64>) -> Self {
        MappedRange {
            ptr,
            range,
            memory: memory.raw(),
            coherent: MaybeCoherent(memory.host_coherent()),
        }
    }

    /// Get raw mapping pointer
    pub fn ptr(&self) -> NonNull<u8> {
        self.ptr
    }

    /// Get raw mapping pointer
    pub fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    /// Fetch readable slice of sub-range to be read.
    /// Invalidating range if memory is not coherent.
    /// `range.end - range.start` must be multiple of `size_of::<T>()`.
    /// `mapping offset + range.start` must be multiple of `align_of::<T>()`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to the memory region for until the borrow ends.
    /// `T` Must be plain-old-data type with memory layout compatible with data written by the device.
    pub unsafe fn read<'b, D, U>(&'b mut self, device: &D, range: Range<u64>) -> Result<&'b [U], MemoryError>
    where
        'a: 'b,
        D: Device<Memory = T>,
        T: Debug + 'static,
        U: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range).ok_or_else(|| MappingError::OutOfBounds)?;

        if self.coherent.0 {
            device.invalidate(Some((self.memory, range.clone())))?;
        }

        let slice = mapped_slice::<U>(ptr, range)?;
        Ok(slice)
    }

    /// Fetch writer to the sub-region.
    /// This writer will flush data on drop if written at least once.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, D, U>(&'b mut self, device: &'b D, range: Range<u64>) -> Result<impl Write<U> + 'b, MappingError>
    where
        'a: 'b,
        D: Device<Memory = T>,
        T: Debug + 'static,
        U: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range).ok_or_else(|| MappingError::OutOfBounds)?;

        if self.coherent.0 {
            device.invalidate(Some((self.memory, range.clone())))?;
        }

        let slice = mapped_slice_mut::<U>(ptr, range.clone())?;

        Ok(WriteFlush {
            slice,
            flush: if self.coherent.0 { Some((device, self.memory, range)) } else { None }
        })
    }
}



