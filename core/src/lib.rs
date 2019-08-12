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

pub use crate::{casts::*, slow::*, wrap::*, backends::*, features::*};

pub use gfx_hal as hal;

rendy_with_empty_backend!{
    pub use gfx_backend_empty as empty;
}

rendy_with_dx12_backend!{
    pub use gfx_backend_dx12 as dx12;
}

rendy_with_gl_backend!{
    pub use gfx_backend_gl as gl;
}

rendy_with_metal_backend!{
    pub use gfx_backend_metal as metal;
}

rendy_with_vulkan_backend!{
    pub use gfx_backend_vulkan as vulkan;
}

with_winit!{
    pub use winit;
}

mod backends;
mod features;
mod casts;
mod slow;
pub mod types;
mod wrap;