mod range;
pub(crate) mod write;

use ash::{version::DeviceV1_0, vk};
use std::{ops::Range, ptr::NonNull};

use error::{MappingError, MemoryError};
use memory::Memory;
use util::fits_usize;

pub(crate) use self::range::{
    mapped_fitting_range, mapped_slice, mapped_slice_mut, mapped_sub_range,
};
use self::write::{Write, WriteCoherent, WriteFlush};

/// Non-coherent marker.
#[derive(Clone, Copy, Debug)]
pub struct NonCoherent;

/// Coherent marker.
#[derive(Clone, Copy, Debug)]
pub struct Coherent;

/// Value that contains either coherent marker or non-coherent marker.
#[derive(Clone, Copy, Debug)]
pub struct MaybeCoherent(bool);

/// Represents range of the memory mapped to the host.
/// Provides methods for safer host access to the memory.
#[derive(Debug)]
pub struct MappedRange<'a, C = MaybeCoherent> {
    /// Memory object that is mapped.
    memory: &'a Memory,

    /// Pointer to range mapped memory.
    ptr: NonNull<u8>,

    /// Range of mapped memory.
    range: Range<u64>,

    /// Coherency marker
    coherent: C,
}

impl<'a> MappedRange<'a> {
    /// Map range of memory.
    ///
    /// # Safety
    ///
    /// * Only one range for the given memory object can be mapped.
    /// * Memory object must be not mapped.
    /// * Memory object must be created with device specified.
    pub unsafe fn new(
        memory: &'a Memory,
        device: &impl DeviceV1_0,
        range: Range<u64>,
    ) -> Result<Self, MappingError> {
        assert!(
            range.start <= range.end,
            "Memory mapping region must have valid size"
        );
        assert!(
            fits_usize(range.end - range.start),
            "Range length must fit in usize"
        );
        assert!(memory.host_visible());

        let ptr = device.map_memory(
            memory.raw(),
            range.start,
            range.end - range.start,
            vk::MemoryMapFlags::empty(),
        )?;
        assert!(
            (ptr as usize).wrapping_neg() <= (range.end - range.start) as usize,
            "Resulting pointer value + range length must fit in usize",
        );

        Ok(Self::from_raw(
            memory,
            NonNull::new_unchecked(ptr as *mut u8),
            range,
        ))
    }

    /// Construct mapped range from raw mapping
    ///
    /// # Safety
    ///
    /// `memory` `range` must be mapped to host memory region pointer by `ptr`.
    pub unsafe fn from_raw(memory: &'a Memory, ptr: NonNull<u8>, range: Range<u64>) -> Self {
        MappedRange {
            ptr,
            range,
            memory,
            coherent: MaybeCoherent(memory.host_coherent()),
        }
    }

    /// Get pointer to beginning of memory region.
    pub fn ptr(&self) -> NonNull<u8> {
        self.ptr
    }

    /// Get mapped range.
    pub fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    /// Fetch readable slice of sub-range to be read.
    /// Invalidating range if memory is not coherent.
    /// `range.end - range.start` must be multiple of `size_of::()`.
    /// `mapping offset + range.start` must be multiple of `align_of::()`.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to the memory region until the borrowing ends.
    /// * `T` Must be plain-old-data type with memory layout compatible with data written by the device.
    pub unsafe fn read<'b, T>(
        &'b mut self,
        device: &impl DeviceV1_0,
        range: Range<u64>,
    ) -> Result<&'b [T], MemoryError>
    where
        'a: 'b,
        T: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range)
            .ok_or_else(|| MappingError::OutOfBounds)?;

        if self.coherent.0 {
            device.invalidate_mapped_memory_ranges(&[vk::MappedMemoryRange::builder()
                .memory(self.memory.raw())
                .offset(self.range.start)
                .size(self.range.end - self.range.start)
                .build()])?;
        }

        let slice = mapped_slice::<T>(ptr, range)?;
        Ok(slice)
    }

    /// Fetch writer to the sub-region.
    /// This writer will flush data on drop if written at least once.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, T>(
        &'b mut self,
        device: &'b impl DeviceV1_0,
        range: Range<u64>,
    ) -> Result<impl Write<T> + 'b, MappingError>
    where
        'a: 'b,
        T: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range)
            .ok_or_else(|| MappingError::OutOfBounds)?;

        if !self.coherent.0 {
            device.invalidate_mapped_memory_ranges(&[vk::MappedMemoryRange::builder()
                .memory(self.memory.raw())
                .offset(self.range.start)
                .size(self.range.end - self.range.start)
                .build()])?;
        }

        let slice = mapped_slice_mut::<T>(ptr, range.clone())?;

        Ok(WriteFlush {
            slice,
            flush: if !self.coherent.0 {
                Some((device, self.memory, range))
            } else {
                None
            },
        })
    }

    /// Convert into mapped range with statically known coherency.
    pub fn coherent(self) -> Result<MappedRange<'a, Coherent>, MappedRange<'a, NonCoherent>> {
        if self.coherent.0 {
            Ok(MappedRange {
                memory: self.memory,
                ptr: self.ptr,
                range: self.range,
                coherent: Coherent,
            })
        } else {
            Err(MappedRange {
                memory: self.memory,
                ptr: self.ptr,
                range: self.range,
                coherent: NonCoherent,
            })
        }
    }
}

impl<'a> From<MappedRange<'a, Coherent>> for MappedRange<'a> {
    fn from(range: MappedRange<'a, Coherent>) -> Self {
        MappedRange {
            memory: range.memory,
            ptr: range.ptr,
            range: range.range,
            coherent: MaybeCoherent(true),
        }
    }
}

impl<'a> From<MappedRange<'a, NonCoherent>> for MappedRange<'a> {
    fn from(range: MappedRange<'a, NonCoherent>) -> Self {
        MappedRange {
            memory: range.memory,
            ptr: range.ptr,
            range: range.range,
            coherent: MaybeCoherent(false),
        }
    }
}

impl<'a> MappedRange<'a, Coherent> {
    /// Fetch writer to the sub-region.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, U>(
        &'b mut self,
        range: Range<u64>,
    ) -> Result<impl Write<U> + 'b, MappingError>
    where
        U: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range)
            .ok_or_else(|| MappingError::OutOfBounds)?;

        let slice = mapped_slice_mut::<U>(ptr, range.clone())?;

        Ok(WriteCoherent { slice })
    }
}
