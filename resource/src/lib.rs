//! This crate provide methods to create/destroy and otherwise manage device resources.
//! Primarily focus on buffers and images.

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
use rendy_descriptor as descriptor;
use rendy_memory as memory;
use rendy_core as core;

mod buffer;
mod escape;
mod image;
mod set;

mod resources;
mod sampler;

pub use crate::{buffer::*, escape::*, image::*, resources::*, sampler::*, set::*};

/// Error creating a resource.
#[derive(Debug)]
pub enum CreationError<E> {
    /// Failed to create an object.
    Create(E),
    /// Failed to allocate memory.
    Allocate(memory::HeapsError),
    /// Failed to bind object memory.
    Bind(rendy_core::hal::device::BindError),
}
