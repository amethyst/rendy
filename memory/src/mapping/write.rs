
use std::{ops::Range, ptr::copy_nonoverlapping};
use device::Device;

pub trait Write<U: Copy> {
    /// Get mutable slice of `U` bound to mapped range.
    /// 
    /// # Safety
    /// 
    /// Slice returned by this function could be hazardous.
    /// User must ensure that bit patterns represents valid values of `U`
    /// or not attempt to read them.
    unsafe fn slice(&mut self) -> &mut [U];

    /// Write data into mapped memory sub-region.
    ///
    /// # Panic
    ///
    /// Panics if `data.len()` is greater than this sub-region len.
    fn write(&mut self, data: &[U]) {
        unsafe {
            let slice = self.slice();
            assert!(data.len() <= slice.len());
            copy_nonoverlapping(data.as_ptr(), slice.as_mut_ptr(), data.len());
        }
    }
}

pub(super) struct WriteFlush<'a, U: 'a, T: 'static, D: Device<Memory = T> + 'a> {
    pub(super) slice: &'a mut [U],
    pub(super) flush: Option<(&'a D, &'a T, Range<u64>)>,
}

impl<'a, U, T, D> Drop for WriteFlush<'a, U, T, D>
where
    U: 'a,
    T: 'static,
    D: Device<Memory = T> + 'a,
{
    fn drop(&mut self) {
        if let Some((device, memory, range)) = self.flush.take() {
            unsafe {
                device.flush(Some((memory, range))).expect("Should flush successfully");
            }
        }
    }
}

impl<'a, U, T, D> Write<U> for WriteFlush<'a, U, T, D>
where
    U: Copy + 'a,
    T: 'a,
    D: Device<Memory = T> + 'a,
{
    unsafe fn slice(&mut self) -> &mut [U] {
        self.slice
    }
}

pub(super) struct WriteCoherent<'a, U: 'a> {
    pub(super) slice: &'a mut [U],
}

impl<'a, U> Write<U> for WriteCoherent<'a, U>
where
    U: Copy + 'a,
{
    unsafe fn slice(&mut self) -> &mut [U] {
        self.slice
    }
}
