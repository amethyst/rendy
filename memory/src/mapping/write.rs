
use std::{ops::Range, ptr::copy_nonoverlapping};

use ash::{version::DeviceV1_0, vk::MappedMemoryRange};
use memory::Memory;


/// Trait for memory region suitable for host writes.
pub trait Write<T: Copy> {
    /// Get mutable slice of `T` bound to mapped range.
    ///
    /// # Safety
    ///
    /// Slice returned by this function could be hazardous.
    /// User must ensure that bit actual patterns represents valid values of `T`
    /// or not attempt to read them.
    unsafe fn slice(&mut self) -> &mut [T];

    /// Write data into mapped memory sub-region.
    ///
    /// # Panic
    ///
    /// Panics if `data.len()` is greater than this sub-region len.
    fn write(&mut self, data: &[T]) {
        unsafe {
            let slice = self.slice();
            assert!(data.len() <= slice.len());
            copy_nonoverlapping(data.as_ptr(), slice.as_mut_ptr(), data.len());
        }
    }
}

#[derive(Debug)]
pub(super) struct WriteFlush<'a, T: 'a, D: DeviceV1_0 + 'a> {
    pub(super) slice: &'a mut [T],
    pub(super) flush: Option<(&'a D, &'a Memory, Range<u64>)>,
}

impl<'a, T, D> Drop for WriteFlush<'a, T, D>
where
    T: 'a,
    D: DeviceV1_0 + 'a,
{
    fn drop(&mut self) {
        if let Some((device, memory, range)) = self.flush.take() {
            unsafe {
                device
                    .flush_mapped_memory_ranges(&[
                        MappedMemoryRange::builder()
                            .memory(memory.raw())
                            .offset(range.start)
                            .size(range.end - range.start)
                            .build(),
                    ])
                    .expect("Should flush successfully");
            }
        }
    }
}

impl<'a, T, D> Write<T> for WriteFlush<'a, T, D>
where
    T: Copy + 'a,
    D: DeviceV1_0 + 'a,
{
    unsafe fn slice(&mut self) -> &mut [T] {
        self.slice
    }
}

#[warn(dead_code)]
#[derive(Debug)]
pub(super) struct WriteCoherent<'a, T: 'a> {
    pub(super) slice: &'a mut [T],
}

impl<'a, T> Write<T> for WriteCoherent<'a, T>
where
    T: Copy + 'a,
{
    unsafe fn slice(&mut self) -> &mut [T] {
        self.slice
    }
}
