use std::borrow::Cow;

pub fn is_slice_sorted<T: Ord>(slice: &[T]) -> bool {
    is_slice_sorted_by_key(slice, |i| i)
}

pub fn is_slice_sorted_by_key<'a, T, K: Ord, F: Fn(&'a T) -> K>(slice: &'a [T], f: F) -> bool {
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

pub fn cast_vec<T>(mut vec: Vec<T>) -> Vec<u8> {
    use std::mem;

    let raw_len = mem::size_of::<T>() * vec.len();
    let len = raw_len;

    let cap = mem::size_of::<T>() * vec.capacity();

    let ptr = vec.as_mut_ptr();
    mem::forget(vec);
    unsafe { Vec::from_raw_parts(ptr as _, len, cap) }
}

pub fn cast_slice<T>(slice: &[T]) -> &[u8] {
    use std::{mem, slice::from_raw_parts};

    let raw_len = mem::size_of::<T>() * slice.len();
    let len = raw_len;

    let ptr = slice.as_ptr();
    mem::forget(slice);
    unsafe { from_raw_parts(ptr as _, len) }
}

pub fn cast_cow<T>(cow: Cow<'_, [T]>) -> Cow<'_, [u8]>
where
    T: Clone,
{
    match cow {
        Cow::Borrowed(slice) => Cow::Borrowed(cast_slice(slice)),
        Cow::Owned(vec) => Cow::Owned(cast_vec(vec)),
    }
}
