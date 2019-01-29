//! This crate provide methods to create/destroy and otherwise manage device resources.
//! Primarily focus on buffers and images.

#![forbid(overflowing_literals)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(intra_doc_link_resolution_failure)]
#![warn(path_statements)]
#![warn(trivial_bounds)]
#![warn(type_alias_bounds)]
#![warn(unconditional_recursion)]
#![warn(unions_with_drop_fields)]
#![warn(while_true)]
#![warn(unused)]
#![warn(bad_style)]
#![warn(future_incompatible)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]
#![allow(unused_unsafe)]

#[macro_use] extern crate derivative;
#[macro_use] extern crate log;
use rendy_memory as memory;

#[doc(hidden)]
pub mod escape;
mod resources;

pub mod buffer;
pub mod image;

pub use crate::{
    escape::KeepAlive,
    buffer::Buffer,
    image::Image,
    resources::Resources,
};
