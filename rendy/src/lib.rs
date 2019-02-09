//! Rendy's top level crate.
//! Reexports all others.

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
#[cfg(feature = "command")]
#[doc(inline)]
pub use rendy_command as command;

#[cfg(feature = "factory")]
#[doc(inline)]
pub use rendy_factory as factory;

#[cfg(feature = "frame")]
#[doc(inline)]
pub use rendy_frame as frame;

#[cfg(feature = "graph")]
#[doc(inline)]
pub use rendy_graph as graph;

#[cfg(feature = "memory")]
#[doc(inline)]
pub use rendy_memory as memory;

#[cfg(feature = "mesh")]
#[doc(inline)]
pub use rendy_mesh as mesh;

#[cfg(feature = "resource")]
#[doc(inline)]
pub use rendy_resource as resource;

#[cfg(feature = "shader")]
#[doc(inline)]
pub use rendy_shader as shader;

#[cfg(feature = "texture")]
#[doc(inline)]
pub use rendy_texture as texture;

#[cfg(feature = "util")]
#[doc(inline)]
pub use rendy_util as util;

#[cfg(feature = "wsi")]
#[doc(inline)]
pub use rendy_wsi as wsi;

pub use gfx_hal as hal;

#[cfg(feature = "gfx-backend-empty")]
pub use gfx_backend_empty as empty;

#[cfg(feature = "gfx-backend-dx12")]
pub use gfx_backend_dx12 as dx12;

#[cfg(feature = "gfx-backend-metal")]
pub use gfx_backend_metal as metal;

#[cfg(feature = "gfx-backend-vulkan")]
pub use gfx_backend_vulkan as vulkan;
