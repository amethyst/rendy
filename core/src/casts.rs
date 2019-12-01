//! Contains functions for casting
use std::{any::TypeId, borrow::Cow, mem::{size_of, align_of}};

/// Cast vec of some arbitrary type into vec of bytes.
/// Can lead to UB if allocator changes. Use with caution.
/// TODO: Replace with something safer.
pub fn cast_vec<T: Copy>(mut vec: Vec<T>) -> Vec<u8> {
    let len = size_of::<T>() * vec.len();
    let cap = size_of::<T>() * vec.capacity();
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec);
    unsafe { Vec::from_raw_parts(ptr as _, len, cap) }
}

/// Cast slice of some arbitrary type into slice of bytes.
pub fn cast_slice<T>(slice: &[T]) -> &[u8] {
    let len = size_of::<T>() * slice.len();
    let ptr = slice.as_ptr();
    unsafe { std::slice::from_raw_parts(ptr as _, len) }
}

/// Cast slice of some arbitrary type into slice of bytes.
pub unsafe fn cast_arbitrary_slice<T, U>(slice: &[T]) -> &[U] {
    let bytes = size_of::<T>() * slice.len();
    let u_s = bytes / size_of::<U>();
    let ptr = slice.as_ptr();
    assert_eq!(ptr as usize % align_of::<U>(), 0);
    std::slice::from_raw_parts(ptr as _, u_s)
}

/// Cast slice of some arbitrary type into slice of bytes.
pub unsafe fn cast_arbitrary_slice_mut<T, U>(slice: &mut [T]) -> &mut [U] {
    let bytes = size_of::<T>() * slice.len();
    let u_s = bytes / size_of::<U>();
    let ptr = slice.as_ptr();
    assert_eq!(ptr as usize % align_of::<U>(), 0);
    std::slice::from_raw_parts_mut(ptr as _, u_s)
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
