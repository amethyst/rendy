//!
//! This crates provides means to deal with vertex buffers and meshes.
//!
//! `Attribute` and `VertexFormat` allow vertex structure to declare semantics.
//! `Mesh` can be created from typed vertex structures and provides mechanism to bind
//! vertex attributes required by shader interface.
//!

#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]
use rendy_command as command;
use rendy_core as core;
use rendy_factory as factory;
use rendy_memory as memory;
use rendy_resource as resource;

mod format;
mod builder;
mod r#static;
mod dynamic;

pub use rendy_core::types::vertex::*;
pub use crate::format::*;
pub use crate::{builder::*, r#static::*, dynamic::*};

fn index_stride(ty: rendy_core::hal::IndexType) -> usize {
    match ty {
        rendy_core::hal::IndexType::U16 => std::mem::size_of::<u16>(),
        rendy_core::hal::IndexType::U32 => std::mem::size_of::<u32>(),
    }
}

fn align_by(align: usize, value: usize) -> usize {
    ((value + align - 1) / align) * align
}

/// Error type returned by `Mesh::bind` in case of mesh's vertex buffers are incompatible with requested vertex formats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatible {
    /// Format that was queried but was not found
    pub not_found: VertexFormat,
    /// List of formats that were available at query time
    pub in_formats: Vec<VertexFormat>,
}

impl std::fmt::Display for Incompatible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Vertex format {:?} is not compatible with any of {:?}.",
            self.not_found, self.in_formats
        )
    }
}
impl std::error::Error for Incompatible {}

/// Check is vertex format `left` is compatible with `right`.
/// `left` must have same `stride` and contain all attributes from `right`.
fn is_compatible(left: &VertexFormat, right: &VertexFormat) -> bool {
    if left.stride != right.stride {
        return false;
    }

    // Don't start searching from index 0 because attributes are sorted
    let mut skip = 0;
    right.attributes.iter().all(|r| {
        left.attributes[skip..]
            .iter()
            .position(|l| l == r)
            .map_or(false, |p| {
                skip += p;
                true
            })
    })
}

/// Chech if slice o f ordered values is sorted.
fn is_slice_sorted<T: Ord>(slice: &[T]) -> bool {
    is_slice_sorted_by_key(slice, |i| i)
}

/// Check if slice is sorted using ordered key and key extractor
fn is_slice_sorted_by_key<'a, T, K: Ord>(slice: &'a [T], f: impl Fn(&'a T) -> K) -> bool {
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

