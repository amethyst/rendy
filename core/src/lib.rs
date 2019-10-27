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

pub use crate::{backend::*, casts::*, slow::*, wrap::*};

#[doc(inline)]
pub use gfx_hal as hal;

#[cfg(all(
    feature = "dx12",
    all(target_os = "windows", not(target_arch = "wasm32"))
))]
#[doc(inline)]
pub use gfx_backend_dx12 as dx12;

#[cfg(all(
    feature = "gl",
    all(target_os = "windows", not(target_arch = "wasm32"))
))]
#[doc(inline)]
pub use gfx_backend_gl as gl;

#[cfg(feature = "gfx-backend-empty")]
#[doc(inline)]
pub use gfx_backend_empty as empty;

#[cfg(all(
    feature = "metal",
    any(all(
        target_os = "macos",
        not(target_arch = "wasm32"),
        all(target_arch = "aarch64", target_os = "ios")
    ))
))]
#[doc(inline)]
pub use gfx_backend_metal as metal;

#[cfg(all(
    feature = "vulkan",
    all(
        any(
            target_os = "windows",
            all(unix, not(any(target_os = "macos", target_os = "ios")))
        ),
        not(target_arch = "wasm32")
    )
))]
#[doc(inline)]
pub use gfx_backend_vulkan as vulkan;

#[doc(inline)]
pub use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

#[macro_use]
mod backend;

#[macro_use]
mod features;
mod casts;
mod slow;
pub mod types;
mod wrap;
