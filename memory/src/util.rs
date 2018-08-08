
use std::ops::Range;

/// usize is less than 64 bit
#[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
pub(crate) fn fits_in_usize(value: u64) -> bool {
    value <= usize::max_value() as u64
}

/// usize is 64 bit or more
#[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))]
pub(crate) fn fits_in_usize(_: u64) -> bool {
    true
}

/// usize is 64 bit or less
#[cfg(any(target_pointer_width = "16", target_pointer_width = "32", target_pointer_width = "64"))]
pub(crate) fn fits_in_u64(_: usize) -> bool {
    true
}

/// usize is more than 64 bit
#[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32", target_pointer_width = "64")))]
pub(crate) fn fits_in_u64(value: usize) -> bool {
    value <= u64::max_value() as usize
}

pub(crate) fn aligned(value: u64, align: u64) -> u64 {
    1 + (value - 1) | (align - 1)
}

/// Check if `sub` fit within `range`.
pub(crate) fn sub_range(range: Range<u64>, sub: Range<u64>) -> bool {
    range.start <= sub.start && range.end >= sub.end
}

pub(crate) fn clamp_range(values: Range<u64>, range: Range<u64>) -> Range<u64> {
    use std::cmp::{min, max};
    min(range.end, max(range.start, values.start)) .. min(range.end, max(range.start, values.end))
}
