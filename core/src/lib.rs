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

#[doc(inline)]
pub use gfx_hal as hal;

#[cfg(feature = "gfx-backend-empty")]
#[doc(inline)]
pub use gfx_backend_empty as empty;

#[cfg(feature = "gfx-backend-dx12")]
#[doc(inline)]
pub use gfx_backend_dx12 as dx12;

#[cfg(feature = "gfx-backend-metal")]
#[doc(inline)]
pub use gfx_backend_metal as metal;

#[cfg(feature = "gfx-backend-vulkan")]
#[doc(inline)]
pub use gfx_backend_vulkan as vulkan;

#[doc(inline)]
pub use raw_window_handle::{RawWindowHandle, HasRawWindowHandle};

#[macro_use]
mod features;
mod casts;
mod slow;
pub mod types;
mod wrap;
