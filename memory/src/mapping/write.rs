use std::{mem::MaybeUninit, ptr::copy_nonoverlapping};

/// Trait for memory region suitable for host writes.
pub trait Write<T: Copy> {
    /// Get mutable slice of `T` bound to mapped range.
    ///
    /// # Safety
    ///
    /// * Returned slice should not be read.
    unsafe fn slice(&mut self) -> &mut [MaybeUninit<T>];

    /// Write data into mapped memory sub-region.
    ///
    /// # Panic
    ///
    /// Panics if `data.len()` is greater than this sub-region len.
    fn write(&mut self, data: &[T]) {
        unsafe {
            let slice = self.slice();
            assert!(data.len() <= slice.len());
            copy_nonoverlapping(data.as_ptr(), slice.as_mut_ptr() as *mut T, data.len());
        }
    }
}

pub(super) struct WriteFlush<'a, T, F: FnOnce() + 'a> {
    pub(super) slice: &'a mut [MaybeUninit<T>],
    pub(super) flush: Option<F>,
}

impl<'a, T, F> Drop for WriteFlush<'a, T, F>
where
    T: 'a,
    F: FnOnce() + 'a,
{
    fn drop(&mut self) {
        if let Some(f) = self.flush.take() {
            f();
        }
    }
}

impl<'a, T, F> Write<T> for WriteFlush<'a, T, F>
where
    T: Copy + 'a,
    F: FnOnce() + 'a,
{
    /// # Safety
    ///
    /// [See doc comment for trait method](trait.Write#method.slice)
    unsafe fn slice(&mut self) -> &mut [MaybeUninit<T>] {
        self.slice
    }
}

pub(super) struct WriteCoherent<'a, T> {
    pub(super) slice: &'a mut [MaybeUninit<T>],
}

impl<'a, T> Write<T> for WriteCoherent<'a, T>
where
    T: Copy + 'a,
{
    /// # Safety
    ///
    /// [See doc comment for trait method](trait.Write#method.slice)
    unsafe fn slice(&mut self) -> &mut [MaybeUninit<T>] {
        self.slice
    }
}
