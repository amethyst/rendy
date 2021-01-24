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

#[cfg(all(
    feature = "dx12",
    all(target_os = "windows", not(target_arch = "wasm32"))
))]
#[doc(inline)]
pub use gfx_backend_dx12 as dx12;
#[cfg(feature = "gfx-backend-empty")]
#[doc(inline)]
pub use gfx_backend_empty as empty;
#[cfg(feature = "gl")]
#[doc(inline)]
pub use gfx_backend_gl as gl;
#[cfg(all(
    feature = "metal",
    any(
        all(not(target_arch = "wasm32"), target_os = "macos"),
        all(target_arch = "aarch64", target_os = "ios")
    )
))]
#[doc(inline)]
pub use gfx_backend_metal as metal;
#[cfg(all(
    feature = "vulkan",
    any(
        target_os = "windows",
        all(unix, not(any(target_os = "macos", target_os = "ios")))
    ),
    not(target_arch = "wasm32")
))]
#[doc(inline)]
pub use gfx_backend_vulkan as vulkan;
#[doc(inline)]
pub use gfx_hal as hal;
#[doc(inline)]
pub use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

pub use crate::{backend::*, casts::*, wrap::*};

#[macro_use]
mod backend;

#[macro_use]
mod features;
mod casts;
pub mod types;
mod wrap;
