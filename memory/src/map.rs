
//! Mapping safety wrappers
//! Mapped memory region can be in following states
//! * Unmapped          - initial state after allocation.
//! * Mapped            - region is mapped by the device to the virtual memory range.
//! * Ready to read     - device does not perform any writes. Range is invalidated for non-coherent memory.
//! * Ready to write    - device does not perform any reads or writes.
//! * Ready to both     - device does not perform any reads or writes. Range is invalidated for non-coherent memory.
//! All except first state can be represented by `MappedRange` with properties inside.
//! To represent any state use `Option<MappedRange>`.

use std::{
    marker::PhantomData,
    mem::{align_of, size_of},
    ops::Range,
    slice::{from_raw_parts, from_raw_parts_mut},
    ptr::{NonNull, copy_nonoverlapping},
};

use either::Either;

use device::Device;
use error::*;
use memory::*;
use util;

pub struct NonCoherent;
pub struct Coherent;
pub type MaybeCoherent = Either<Coherent, NonCoherent>;

pub(crate) fn maybe_coherent(coherent: bool) -> MaybeCoherent {
    if coherent {
        Either::Left(Coherent)
    } else {
        Either::Right(NonCoherent)
    }
}

pub struct MappedRange<'a, T: 'a, C = MaybeCoherent> {
    /// Pointer to the mapped range.
    pub(crate) ptr: NonNull<u8>,

    /// Memory object that is mapped.
    pub(crate) memory: &'a T,

    /// Offset of mapped range. `ptr` points to this offset.
    pub(crate) offset: u64,

    /// Length of mapped range.
    pub(crate) length: usize,

    pub(crate) coherent: C,
}

impl<'a, T: 'a, C> MappedRange<'a, T, C> {
    #[inline(always)]
    fn check_range<U>(&self, range: &Range<usize>) {
        debug_assert!(util::fits_in_u64(self.length), "Can't map out of u64 size");
        debug_assert!(util::fits_in_u64(align_of::<U>()));
        debug_assert!(util::fits_in_u64(size_of::<U>()));
        assert!(range.start <= range.end);
        assert!(range.end <= self.length);
        assert_eq!((self.offset + range.start as u64) % align_of::<U>() as u64, 0);
        assert_eq!((range.end as u64 - range.start as u64) % size_of::<U>() as u64, 0);
    }
}

impl<'a, T: 'a> MappedRange<'a, T, MaybeCoherent> {
    /// Fetch readable slice of sub-range to be read.
    /// Invalidating range if memory is not coherent.
    /// `range.end - range.start` must be multiple of `size_of::<T>()`.
    /// `mapping offset + range.start` must be multiple of `align_of::<T>()`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to the memory region for until the borrow ends.
    /// `T` Must be plain-old-data type with memory layout compatible with data written by the device.
    pub unsafe fn read<'b, D, U>(&'b mut self, device: &D, range: Range<usize>) -> Result<&'b [U], OutOfMemoryError>
    where
        'a: 'b,
        D: Device<T>,
    {
        self.check_range::<U>(&range);

        if self.coherent.is_right() {
            device.invalidate(Some((self.memory, (self.offset + range.start as u64 .. self.offset + range.end as u64))))?;
        }

        Ok(from_raw_parts(self.ptr.as_ptr().add(range.start) as *const U, range.end - range.start))
    }

    /// Fetch writer to the sub-region.
    /// This writer will flush data on drop if written at least once.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, D, U>(&'b mut self, device: &'b D, range: Range<usize>) -> impl Write<U> + 'b
    where
        'a: 'b,
        D: Device<T>,
    {
        self.check_range::<U>(&range);

        WriteFlush {
            ptr: NonNull::new_unchecked(self.ptr.as_ptr().add(range.start)).cast::<U>(),
            length: range.end - range.start,
            marker: PhantomData,
            flush: if self.coherent.is_right() { Some((device, self.memory, self.offset + range.start as u64)) } else { None }
        }
    }
}

impl<'a, T: 'a> MappedRange<'a, T, NonCoherent> {
    /// Fetch readable slice of sub-range to be read.
    /// Invalidating range if memory is not coherent.
    /// `range.end - range.start` must be multiple of `size_of::<T>()`.
    /// `mapping offset + range.start` must be multiple of `align_of::<T>()`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to the memory region for until the borrow ends.
    /// `T` Must be plain-old-data type with memory layout compatible with data written by the device.
    pub unsafe fn read<'b, D, U>(&'b mut self, device: &D, range: Range<usize>) -> Result<&'b [U], OutOfMemoryError>
    where
        'a: 'b,
        D: Device<T>,
    {
        self.check_range::<U>(&range);
        device.invalidate(Some((self.memory, (self.offset + range.start as u64 .. self.offset + range.end as u64))))?;
        Ok(from_raw_parts(self.ptr.as_ptr().add(range.start) as *const U, range.end - range.start))
    }

    /// Fetch writer to the sub-region.
    /// This writer will flush data on drop if written at least once.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, D, U>(&'b mut self, device: &'b D, range: Range<usize>) -> impl Write<U> + 'b
    where
        'a: 'b,
        D: Device<T>,
    {
        self.check_range::<U>(&range);

        WriteFlush {
            ptr: NonNull::new_unchecked(self.ptr.as_ptr().add(range.start)).cast::<U>(),
            length: range.end - range.start,
            marker: PhantomData,
            flush: Some((device, self.memory, self.offset + range.start as u64))
        }
    }
}

impl<'a, T: 'a> MappedRange<'a, T, Coherent> {
    /// Fetch readable slice of sub-range to be read.
    /// Invalidating range if memory is not coherent.
    /// `range.end - range.start` must be multiple of `size_of::<T>()`.
    /// `mapping offset + range.start` must be multiple of `align_of::<T>()`.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to the memory region for until the borrow ends.
    /// `T` Must be plain-old-data type with memory layout compatible with data written by the device.
    pub unsafe fn read<'b, D, U>(&'b mut self, range: Range<usize>) -> &'b [U]
    where
        'a: 'b,
        D: Device<T>,
    {
        self.check_range::<U>(&range);
        from_raw_parts(self.ptr.as_ptr().add(range.start) as *const U, range.end - range.start)
    }

    /// Fetch writer to the sub-region.
    /// This writer will flush data on drop if written at least once.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure that device won't write to or read from the memory region.
    pub unsafe fn write<'b, U>(&'b mut self, range: Range<usize>) -> impl Write<U> + 'b
    where
        'a: 'b,
    {
        self.check_range::<U>(&range);

        WriteCoherent {
            ptr: NonNull::new_unchecked(self.ptr.as_ptr().add(range.start)).cast::<U>(),
            length: range.end - range.start,
            marker: PhantomData,
        }
    }
}

pub trait Write<U> {
    /// Get pointer to the writtable sub-region.
    /// Caller must take care to not write via returned pointer after dropping this `Write` instance.
    fn ptr(&self) -> NonNull<U>;

    /// Get len of the memory range.
    fn len(&self) -> usize;

    /// Write data into writtable mapped memory sub-region.
    ///
    /// # Panic
    ///
    /// Panics if `data.len()` is greater than this sub-region len.
    fn write(&mut self, data: &[U]) {
        unsafe {
            assert!(data.len() <= self.len());
            copy_nonoverlapping(data.as_ptr(), self.ptr().as_ptr(), data.len());
        }
    }
}

struct WriteFlush<'a, U: 'a, T: 'a, D: Device<T> + 'a> {
    ptr: NonNull<U>,
    length: usize,
    marker: PhantomData<&'a mut [U]>,
    flush: Option<(&'a D, &'a T, u64)>,
}

impl<'a, U, T, D> Drop for WriteFlush<'a, U, T, D>
where
    U: 'a,
    T: 'a,
    D: Device<T> + 'a,
{
    fn drop(&mut self) {
        debug_assert!(util::fits_in_u64(self.length));
        if let Some((device, memory, offset)) = self.flush.take() {
            unsafe {
                device.flush(Some((memory, offset .. offset + self.length as u64))).expect("Should flush successfully");
            }
        }
    }
}

impl<'a, U, T, D> Write<U> for WriteFlush<'a, U, T, D>
where
    U: 'a,
    T: 'a,
    D: Device<T> + 'a,
{
    fn ptr(&self) -> NonNull<U> {
        self.ptr
    }

    fn len(&self) -> usize {
        self.length
    }
}

struct WriteCoherent<'a, U: 'a> {
    ptr: NonNull<U>,
    length: usize,
    marker: PhantomData<&'a mut [U]>,
}

impl<'a, U> Write<U> for WriteCoherent<'a, U>
where
    U: 'a,
{
    fn ptr(&self) -> NonNull<U> {
        self.ptr
    }

    fn len(&self) -> usize {
        self.length
    }
}
