//! Contains functions for casting
use std::{any::TypeId, borrow::Cow, mem, slice};

/// Cast vec of some arbitrary type into vec of bytes.
/// Can lead to UB if allocator changes. Use with caution.
/// TODO: Replace with something safer.
pub fn cast_vec<T: Copy>(mut vec: Vec<T>) -> Vec<u8> {
    let len = std::mem::size_of::<T>() * vec.len();
    let cap = std::mem::size_of::<T>() * vec.capacity();
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec);
    unsafe { Vec::from_raw_parts(ptr as _, len, cap) }
}

/// Cast slice of some arbitrary type into slice of bytes.
pub fn cast_slice<T>(slice: &[T]) -> &[u8] {
    let len = std::mem::size_of::<T>() * slice.len();
    let ptr = slice.as_ptr();
    unsafe { std::slice::from_raw_parts(ptr as _, len) }
}

/// Cast `cow` of some arbitrary type into `cow` of bytes.
/// Can lead to UB if allocator changes. Use with caution.
/// TODO: Replace with something safer.
pub fn cast_cow<T: Copy>(cow: Cow<'_, [T]>) -> Cow<'_, [u8]> {
    match cow {
        Cow::Borrowed(slice) => Cow::Borrowed(cast_slice(slice)),
        Cow::Owned(vec) => Cow::Owned(cast_vec(vec)),
    }
}

/// Casts identical types.
/// Useful in generic environment where caller knows that two types are the same
/// but Rust is not convinced.
///
/// # Panics
///
/// Panics if types are actually different.
///
/// # Example
///
/// ```
/// # extern crate rendy_core;
/// # use rendy_core::identical_cast;
/// # use std::any::TypeId;
/// # fn foo<T: 'static>() {
/// if TypeId::of::<T>() == TypeId::of::<u32>() {
///     let value: T = identical_cast(42u32);
/// }
/// # }
///
/// ```
pub fn identical_cast<T: 'static, U: 'static>(value: T) -> U {
    assert_eq!(TypeId::of::<T>(), TypeId::of::<U>());
    unsafe {
        // We know types are the same.
        let mut value = std::mem::ManuallyDrop::new(value);
        let ptr: *mut T = &mut *value;
        std::ptr::read(ptr as *mut U)
    }
}

/// Tries to cast a slice from one type to another.
///
/// Can fail and return `None` if the from slice is not sized or aligned correctly.
///
/// Safety
/// ======
///
/// Must be able to interpret `To` from the bytes in `From` safely (you could, for example, create invalid references).
pub unsafe fn cast_any_slice<From: 'static + Copy, To: 'static + Copy>(
    from: &[From],
) -> Option<&[To]> {
    let from_size = mem::size_of::<From>();
    let from_ptr = from.as_ptr();
    let from_len = from.len();
    let to_size = mem::size_of::<To>();
    let to_align = mem::align_of::<To>();
    if from_size == 0 || to_size == 0 {
        // can't cast zero-sized types
        return None;
    }
    if (from_len * from_size) % to_size != 0 {
        // invalid size
        return None;
    }
    if from_ptr.align_offset(to_align) != 0 {
        // unaligned pointer
        return None;
    }
    let to_len = (from_len * from_size) / to_size;
    Some(slice::from_raw_parts(from_ptr as *const To, to_len))
}
