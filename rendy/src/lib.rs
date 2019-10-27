//! Rendy's top level crate.
//! Reexports all others.

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

#[doc(inline)]
pub use rendy_core as core;

#[doc(inline)]
pub use rendy_core::hal;

#[cfg(feature = "empty")]
#[doc(inline)]
pub use rendy_core::empty;

#[cfg(feature = "dx12")]
#[doc(inline)]
pub use rendy_core::dx12;

#[cfg(feature = "metal")]
#[doc(inline)]
pub use rendy_core::metal;

#[cfg(feature = "vulkan")]
#[doc(inline)]
pub use rendy_core::vulkan;

#[cfg(feature = "command")]
#[doc(inline)]
pub use rendy_command as command;

#[cfg(feature = "descriptor")]
#[doc(inline)]
pub use rendy_descriptor as descriptor;

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

#[cfg(feature = "wsi")]
#[doc(inline)]
pub use rendy_wsi as wsi;
