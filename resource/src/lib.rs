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
use rendy_descriptor as descriptor;
use rendy_memory as memory;

pub mod buffer;
pub mod escape;
pub mod image;
pub mod resources;
pub mod sampler;
pub mod set;

pub use crate::{
    buffer::Buffer,
    escape::{Escape, Handle},
    image::{Image, ImageView},
    resources::ResourceTracker,
    sampler::Sampler,
    set::{DescriptorSet, DescriptorSetLayout},
};

#[doc(hidden)]
pub use crate::resources::Epochs;
