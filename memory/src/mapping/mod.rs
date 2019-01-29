mod range;
pub(crate) mod write;

use std::{ops::Range, ptr::NonNull};

use crate::{
    memory::Memory,
    util::fits_usize,
};

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
pub struct MappedRange<'a, B: gfx_hal::Backend, C = MaybeCoherent> {
    /// Memory object that is mapped.
    memory: &'a Memory<B>,

    /// Pointer to range mapped memory.
    ptr: NonNull<u8>,

    /// Range of mapped memory.
    range: Range<u64>,

    /// Coherency marker
    coherent: C,
}

impl<'a, B> MappedRange<'a, B>
where
    B: gfx_hal::Backend,
{
    /// Map range of memory.
    ///
    /// # Safety
    ///
    /// * Only one range for the given memory object can be mapped.
    /// * Memory object must be not mapped.
    /// * Memory object must be created with device specified.
    pub unsafe fn new(
        memory: &'a Memory<B>,
        device: &impl gfx_hal::Device<B>,
        range: Range<u64>,
    ) -> Result<Self, gfx_hal::mapping::Error> {
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
            range.clone(),
        )?;
        assert!(
            (ptr as usize).wrapping_neg() >= (range.end - range.start) as usize,
            "Resulting pointer value + range length must fit in usize. Pointer: {:p}, range {:?}", ptr, range,
        );

        Ok(Self::from_raw(
            memory,
            NonNull::new_unchecked(ptr),
            range,
        ))
    }

    /// Construct mapped range from raw mapping
    ///
    /// # Safety
    ///
    /// `memory` `range` must be mapped to host memory region pointer by `ptr`.
    pub unsafe fn from_raw(memory: &'a Memory<B>, ptr: NonNull<u8>, range: Range<u64>) -> Self {
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
        device: &impl gfx_hal::Device<B>,
        range: Range<u64>,
    ) -> Result<&'b [T], gfx_hal::mapping::Error>
    where
        'a: 'b,
        T: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range)
            .ok_or_else(|| gfx_hal::mapping::Error::OutOfBounds)?;

        if self.coherent.0 {
            device.invalidate_mapped_memory_ranges(
                Some((self.memory.raw(), self.range.clone()))
            )?;
        }

        let slice = mapped_slice::<T>(ptr, range);
        Ok(slice)
    }

    /// Fetch writer to the sub-region.
    /// This writer will flush data on drop if written at least once.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, T: 'b>(
        &'b mut self,
        device: &'b impl gfx_hal::Device<B>,
        range: Range<u64>,
    ) -> Result<impl Write<T> + 'b, gfx_hal::mapping::Error>
    where
        'a: 'b,
        T: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range)
            .ok_or_else(|| gfx_hal::mapping::Error::OutOfBounds)?;

        if !self.coherent.0 {
            device.invalidate_mapped_memory_ranges(
                Some((self.memory.raw(), self.range.clone()))
            )?;
        }

        let slice = mapped_slice_mut::<T>(ptr, range.clone());

        let ref memory = self.memory;

        Ok(WriteFlush {
            slice,
            flush: if !self.coherent.0 {
                Some(move || {
                    device.flush_mapped_memory_ranges(Some((memory.raw(), range)))
                        .expect("Should flush successfully");
                })
            } else {
                None
            }
        })
    }

    /// Convert into mapped range with statically known coherency.
    pub fn coherent(self) -> Result<MappedRange<'a, B, Coherent>, MappedRange<'a, B, NonCoherent>> {
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

impl<'a, B> From<MappedRange<'a, B, Coherent>> for MappedRange<'a, B>
where
    B: gfx_hal::Backend,
{
    fn from(range: MappedRange<'a, B, Coherent>) -> Self {
        MappedRange {
            memory: range.memory,
            ptr: range.ptr,
            range: range.range,
            coherent: MaybeCoherent(true),
        }
    }
}

impl<'a, B> From<MappedRange<'a, B, NonCoherent>> for MappedRange<'a, B>
where
    B: gfx_hal::Backend,
{
    fn from(range: MappedRange<'a, B, NonCoherent>) -> Self {
        MappedRange {
            memory: range.memory,
            ptr: range.ptr,
            range: range.range,
            coherent: MaybeCoherent(false),
        }
    }
}

impl<'a, B> MappedRange<'a, B, Coherent>
where
    B: gfx_hal::Backend,
{
    /// Fetch writer to the sub-region.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, U: 'b>(
        &'b mut self,
        range: Range<u64>,
    ) -> Result<impl Write<U> + 'b, gfx_hal::mapping::Error>
    where
        U: Copy,
    {
        let (ptr, range) = mapped_sub_range(self.ptr, self.range.clone(), range)
            .ok_or_else(|| gfx_hal::mapping::Error::OutOfBounds)?;

        let slice = mapped_slice_mut::<U>(ptr, range.clone());

        Ok(WriteCoherent { slice })
    }
}
