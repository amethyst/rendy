//! This crate provide methods to create/destroy and otherwise manage device resources.
//! Primarily focus on buffers and images.

#![forbid(overflowing_literals)]
#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]
#![deny(path_statements)]
#![deny(trivial_bounds)]
#![deny(type_alias_bounds)]
#![deny(unconditional_recursion)]
#![deny(unions_with_drop_fields)]
#![deny(while_true)]
#![deny(unused)]
#![deny(bad_style)]
#![deny(future_incompatible)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![allow(unused_unsafe)]

#[macro_use] extern crate derivative;
#[macro_use] extern crate log;
use rendy_memory as memory;

mod escape;
mod resources;

pub mod buffer;
pub mod image;

pub use crate::{
    escape::KeepAlive,
    buffer::Buffer,
    image::Image,
    resources::Resources,
};
