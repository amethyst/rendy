//!
//! 

#![forbid(overflowing_literals)]
#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]
#![deny(path_statements)]
#![deny(trivial_bounds)]
#![deny(type_alias_bounds)]
#![deny(unconditional_recursion)]
#![deny(unions_with_drop_fields)]
#![deny(while_true)]
#![deny(unused)]
#![deny(bad_style)]
#![deny(future_incompatible)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]

use std::borrow::Cow;

/// Chech if slice o f ordered values is sorted.
pub fn is_slice_sorted<T: Ord>(slice: &[T]) -> bool {
    is_slice_sorted_by_key(slice, |i| i)
}

/// Check if slice is sorted using ordered key and key extractor
pub fn is_slice_sorted_by_key<'a, T, K: Ord>(slice: &'a [T], f: impl Fn(&'a T) -> K) -> bool {
    if let Some((first, slice)) = slice.split_first() {
        let mut cmp = f(first);
        for item in slice {
            let item = f(item);
            if cmp > item {
                return false;
            }
            cmp = item;
        }
    }
    true
}

/// Cast vec of some arbitrary type into vec of bytes.
pub fn cast_vec<T>(mut vec: Vec<T>) -> Vec<u8> {
    use std::mem;

    let raw_len = mem::size_of::<T>() * vec.len();
    let len = raw_len;

    let cap = mem::size_of::<T>() * vec.capacity();

    let ptr = vec.as_mut_ptr();
    mem::forget(vec);
    unsafe { Vec::from_raw_parts(ptr as _, len, cap) }
}

/// Cast slice of some arbitrary type into slice of bytes.
pub fn cast_slice<T>(slice: &[T]) -> &[u8] {
    use std::{mem, slice::from_raw_parts};

    let raw_len = mem::size_of::<T>() * slice.len();
    let len = raw_len;

    let ptr = slice.as_ptr();
    unsafe { from_raw_parts(ptr as _, len) }
}

/// Cast `cow` of some arbitrary type into `cow` of bytes.
pub fn cast_cow<T>(cow: Cow<'_, [T]>) -> Cow<'_, [u8]>
where
    T: Clone,
{
    match cow {
        Cow::Borrowed(slice) => Cow::Borrowed(cast_slice(slice)),
        Cow::Owned(vec) => Cow::Owned(cast_vec(vec)),
    }
}
