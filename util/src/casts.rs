//! Contains functions for casting
use std::{any::TypeId, borrow::Cow};

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

/// Safely turn a slice of bytes into a `Vec<u32>`,
/// intended to make it easy to load SPIR-V bytecode
/// from a file.  Copies its input, since a `&[u32]`
/// has aligment constraints that `&[u8]` may not
/// fulfill.  Always assumes native endianness,
/// since SPIR-V is defined to be endian-independent.
/// Panics if the input length does not divide evenly by 4.
///
/// TODO: Make it return `Cow<'a, [u32]>` and
/// only copy the input if necessary.
pub fn cast_spirv_bytes(bytes: &[u8]) -> Vec<u32> {
    assert!(
        bytes.len() % 4 != 0,
        "cast_spirv_bytes() got input of a length that doesn't fit into u32's!"
    );
    let mut accm = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let mut arr: [u8; 4] = [0; 4];
        arr.copy_from_slice(chunk);
        let i = u32::from_ne_bytes(arr);
        accm.push(i);
    }
    accm
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
/// # extern crate rendy_util;
/// # use rendy_util::identical_cast;
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
