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
#![allow(clippy::self_named_constructors)]
use rendy_core as core;
use rendy_descriptor as descriptor;
use rendy_memory as memory;

mod buffer;
mod escape;
mod image;
mod set;

mod resources;
mod sampler;

pub use crate::{buffer::*, escape::*, image::*, resources::*, sampler::*, set::*};

/// Error creating a resource.
#[derive(Clone, Debug, PartialEq)]
pub enum CreationError<E> {
    /// Failed to create an object.
    Create(E),
    /// Failed to allocate memory.
    Allocate(memory::HeapsError),
    /// Failed to bind object memory.
    Bind(rendy_core::hal::device::BindError),
}

impl<E> std::fmt::Display for CreationError<E>
where
    E: std::fmt::Debug,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreationError::Create(_err) => write!(fmt, "Failed to create object"), // Uncomment after gfx-0.4.1 std::fmt::Display::fmt(err, fmt),
            CreationError::Allocate(err) => write!(fmt, "Failed to create object: {}", err),
            CreationError::Bind(err) => write!(fmt, "Failed to create object: {:?}", err),
        }
    }
}

impl<E> std::error::Error for CreationError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CreationError::Create(err) => Some(err),
            CreationError::Allocate(err) => Some(err),
            CreationError::Bind(err) => Some(err),
        }
    }
}
