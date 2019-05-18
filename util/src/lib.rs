//! Crate that contains utility modules used by other rendy crates

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

pub use crate::{casts::*, slow::*, wrap::*};

#[cfg(feature = "gfx-backend-empty")]
pub use gfx_backend_empty as empty;

#[cfg(feature = "gfx-backend-dx12")]
pub use gfx_backend_dx12 as dx12;

#[cfg(feature = "gfx-backend-metal")]
pub use gfx_backend_metal as metal;

#[cfg(feature = "gfx-backend-vulkan")]
pub use gfx_backend_vulkan as vulkan;

#[macro_use]
mod features;
mod casts;
mod slow;
pub mod types;
mod wrap;
