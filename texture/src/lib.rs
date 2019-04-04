//!
//! Texture creation and usage.
//!
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
use rendy_factory as factory;
use rendy_memory as memory;
use rendy_resource as resource;
use rendy_util as util;

mod format;
pub mod pixel;
mod texture;

pub use crate::{format::*, pixel::Rgba8Unorm, texture::*};
