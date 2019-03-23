//! This crate provide methods to create/destroy and otherwise manage device resources.
//! Primarily focus on buffers and images.

#[warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate log;
use rendy_descriptor as descriptor;
use rendy_memory as memory;

#[doc(hidden)]
pub mod escape;
mod resources;

pub mod buffer;
pub mod image;
pub mod sampler;
pub mod set;

pub use crate::{
    buffer::Buffer, escape::KeepAlive, image::Image, resources::Resources, set::DescriptorSet,
};

#[doc(hidden)]
pub use crate::resources::Epochs;
