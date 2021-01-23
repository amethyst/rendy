mod range;
pub(crate) mod write;

use {
    crate::{memory::Memory, util::*},
    gfx_hal::{device::Device as _, Backend},
    std::{ops::Range, ptr::NonNull},
};

pub(crate) use self::range::*;
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
pub struct MappedRange<'a, B: Backend, C = MaybeCoherent> {
    /// Memory object that is mapped.
    memory: &'a Memory<B>,

    /// Pointer to range mapped memory.
    ptr: NonNull<u8>,

    /// Range of mapped memory.
    mapping_range: Range<u64>,

    /// Mapping range requested by caller.
    /// Must be subrange of `mapping_range`.
    requested_range: Range<u64>,

    /// Coherency marker
    coherent: C,
}

impl<'a, B> MappedRange<'a, B>
where
    B: Backend,
{
    // /// Map range of memory.
    // /// `range` is in memory object space.
    // ///
    // /// # Safety
    // ///
    // /// * Only one range for the given memory object can be mapped.
    // /// * Memory object must be not mapped.
    // /// * Memory object must be created with device specified.
    // pub unsafe fn new(
    //     memory: &'a Memory<B>,
    //     device: &B::Device,
    //     range: Range<u64>,
    // ) -> Result<Self, gfx_hal::device::MapError> {

    //     let ptr = device.map_memory(memory.raw(), range.clone())?;

    //     Ok(Self::from_raw(memory, NonNull::new_unchecked(ptr), range))
    // }

    /// Construct mapped range from raw mapping
    ///
    /// # Safety
    ///
    /// `memory` `range` must be mapped to host memory region pointer by `ptr`.
    /// `range` is in memory object space.
    /// `ptr` points to the `range.start` offset from memory origin.
    pub(crate) unsafe fn from_raw(
        memory: &'a Memory<B>,
        ptr: NonNull<u8>,
        mapping_range: Range<u64>,
        requested_range: Range<u64>,
    ) -> Self {
        MappedRange {
            ptr,
            mapping_range,
            requested_range,
            memory,
            coherent: MaybeCoherent(memory.host_coherent()),
        }
    }

    /// Get pointer to beginning of memory region.
    /// i.e. to `range().start` offset from memory origin.
    pub fn ptr(&self) -> NonNull<u8> {
        mapped_sub_range(
            self.ptr,
            self.mapping_range.clone(),
            self.requested_range.clone(),
        )
        .unwrap()
    }

    /// Get mapped range.
    pub fn range(&self) -> Range<u64> {
        self.requested_range.clone()
    }

    /// Fetch readable slice of sub-range to be read.
    /// Invalidating range if memory is not coherent.
    /// `range.end - range.start` must be multiple of `size_of::()`.
    /// `mapping offset + range.start` must be multiple of `align_of::()`.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to the memory region until the borrowing ends.
    /// * `T` Must be plain-old-data type compatible with data in mapped region.
    pub unsafe fn read<'b, T>(
        &'b mut self,
        device: &B::Device,
        range: Range<u64>,
    ) -> Result<&'b [T], gfx_hal::device::MapError>
    where
        'a: 'b,
        T: Copy,
    {
        let sub_range = relative_to_sub_range(self.requested_range.clone(), range)
            .ok_or(gfx_hal::device::MapError::OutOfBounds)?;

        let ptr =
            mapped_sub_range(self.ptr, self.mapping_range.clone(), sub_range.clone()).unwrap();

        let size = (sub_range.end - sub_range.start) as usize;

        if !self.coherent.0 {
            let aligned_sub_range = align_range(sub_range, self.memory.non_coherent_atom_size());
            device.invalidate_mapped_memory_ranges(Some((
                self.memory.raw(),
                gfx_hal::memory::Segment {
                    offset: aligned_sub_range.start,
                    size: Some(aligned_sub_range.end - aligned_sub_range.start),
                },
            )))?;
        }

        let slice = mapped_slice::<T>(ptr, size);
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
        device: &'b B::Device,
        range: Range<u64>,
    ) -> Result<impl Write<T> + 'b, gfx_hal::device::MapError>
    where
        'a: 'b,
        T: Copy,
    {
        let sub_range = relative_to_sub_range(self.requested_range.clone(), range)
            .ok_or(gfx_hal::device::MapError::OutOfBounds)?;

        let ptr =
            mapped_sub_range(self.ptr, self.mapping_range.clone(), sub_range.clone()).unwrap();

        let size = (sub_range.end - sub_range.start) as usize;

        let slice = mapped_slice_mut::<T>(ptr, size);

        let memory = &self.memory;
        let flush = if !self.coherent.0 {
            let aligned_sub_range = align_range(sub_range, self.memory.non_coherent_atom_size());
            Some(move || {
                device
                    .flush_mapped_memory_ranges(Some((
                        memory.raw(),
                        gfx_hal::memory::Segment {
                            offset: aligned_sub_range.start,
                            size: Some(aligned_sub_range.end - aligned_sub_range.start),
                        },
                    )))
                    .expect("Should flush successfully");
            })
        } else {
            None
        };

        Ok(WriteFlush { slice, flush })
    }

    /// Convert into mapped range with statically known coherency.
    pub fn coherent(self) -> Result<MappedRange<'a, B, Coherent>, MappedRange<'a, B, NonCoherent>> {
        if self.coherent.0 {
            Ok(MappedRange {
                memory: self.memory,
                ptr: self.ptr,
                mapping_range: self.mapping_range,
                requested_range: self.requested_range,
                coherent: Coherent,
            })
        } else {
            Err(MappedRange {
                memory: self.memory,
                ptr: self.ptr,
                mapping_range: self.mapping_range,
                requested_range: self.requested_range,
                coherent: NonCoherent,
            })
        }
    }
}

impl<'a, B> From<MappedRange<'a, B, Coherent>> for MappedRange<'a, B>
where
    B: Backend,
{
    fn from(range: MappedRange<'a, B, Coherent>) -> Self {
        MappedRange {
            memory: range.memory,
            ptr: range.ptr,
            mapping_range: range.mapping_range,
            requested_range: range.requested_range,
            coherent: MaybeCoherent(true),
        }
    }
}

impl<'a, B> From<MappedRange<'a, B, NonCoherent>> for MappedRange<'a, B>
where
    B: Backend,
{
    fn from(range: MappedRange<'a, B, NonCoherent>) -> Self {
        MappedRange {
            memory: range.memory,
            ptr: range.ptr,
            mapping_range: range.mapping_range,
            requested_range: range.requested_range,
            coherent: MaybeCoherent(false),
        }
    }
}

impl<'a, B> MappedRange<'a, B, Coherent>
where
    B: Backend,
{
    /// Fetch writer to the sub-region.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, U: 'b>(
        &'b mut self,
        range: Range<u64>,
    ) -> Result<impl Write<U> + 'b, gfx_hal::device::MapError>
    where
        U: Copy,
    {
        let sub_range = relative_to_sub_range(self.requested_range.clone(), range)
            .ok_or(gfx_hal::device::MapError::OutOfBounds)?;

        let ptr =
            mapped_sub_range(self.ptr, self.mapping_range.clone(), sub_range.clone()).unwrap();

        let size = (sub_range.end - sub_range.start) as usize;

        let slice = mapped_slice_mut::<U>(ptr, size);

        Ok(WriteCoherent { slice })
    }
}
