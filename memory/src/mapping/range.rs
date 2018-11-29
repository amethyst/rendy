use std::{
    mem::{align_of, size_of},
    ops::Range,
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

/// Get sub-range of memory mapping.
/// `range` is in memory object space.
pub(crate) fn mapped_fitting_range(
    ptr: NonNull<u8>,
    range: Range<u64>,
    fitting: Range<u64>,
) -> Option<NonNull<u8>> {
    assert!(
        range.start <= range.end,
        "Memory mapping region must have valid size"
    );
    assert!(
        fitting.start <= fitting.end,
        "Memory mapping region must have valid size"
    );
    debug_assert!(range.start <= range.end);

    if fitting.start < range.start || fitting.end > range.end {
        None
    } else {
        Some(unsafe {
            // for x > 0 and y >= 0: x + y > 0. No overlapping due to assertions in `new` and checks above.
            NonNull::new_unchecked(
                (ptr.as_ptr() as usize + (fitting.start - range.start) as usize) as *mut u8,
            )
        })
    }
}

/// Get sub-range of memory mapping.
/// `range` is in mapping object space.
pub(crate) fn mapped_sub_range(
    ptr: NonNull<u8>,
    range: Range<u64>,
    sub: Range<u64>,
) -> Option<(NonNull<u8>, Range<u64>)> {
    let fitting = sub.start.checked_add(range.start)?..sub.end.checked_add(range.start)?;
    let ptr = mapped_fitting_range(ptr, range, fitting.clone())?;
    Some((ptr, fitting))
}

/// # Safety
///
/// User must ensure that:
/// * this function won't create aliasing slices.
/// * returned slice doesn't outlive mapping.
pub(crate) unsafe fn mapped_slice_mut<'a, T>(
    ptr: NonNull<u8>,
    range: Range<u64>,
) -> &'a mut [T] {
    let size = (range.end - range.start) as usize;
    assert_eq!(
        size % size_of::<T>(),
        0,
        "Range length must be multiple of element size"
    );
    let offset = ptr.as_ptr() as usize;
    assert_eq!(offset % align_of::<T>(), 0, "Range offset must be multiple of element alignment");
    from_raw_parts_mut(ptr.as_ptr() as *mut T, size)
}

/// # Safety
///
/// User must ensure that:
/// * returned slice doesn't outlive mapping.
pub(crate) unsafe fn mapped_slice<'a, T>(
    ptr: NonNull<u8>,
    range: Range<u64>,
) -> &'a [T] {
    let size = (range.end - range.start) as usize;
    assert_eq!(
        size % size_of::<T>(),
        0,
        "Range length must be multiple of element size"
    );
    let offset = ptr.as_ptr() as usize;
    assert_eq!(offset % align_of::<T>(), 0, "Range offset must be multiple of element alignment");
    from_raw_parts(ptr.as_ptr() as *const T, size)
}
