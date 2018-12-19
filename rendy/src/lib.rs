
//! Rendy's top level crate.
//! Reexports all others.

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

#[doc(inline)] pub use rendy_command as command;
#[doc(inline)] pub use rendy_factory as factory;
#[doc(inline)] pub use rendy_frame as frame;
#[doc(inline)] pub use rendy_graph as graph;
#[doc(inline)] pub use rendy_memory as memory;
#[doc(inline)] pub use rendy_mesh as mesh;
#[doc(inline)] pub use rendy_resource as resource;
#[doc(inline)] pub use rendy_shader as shader;
#[doc(inline)] pub use rendy_wsi as wsi;

pub use gfx_hal as hal;

#[cfg(feature = "gfx-backend-empty")]
pub use gfx_backend_empty as empty;

#[cfg(feature = "gfx-backend-dx12")]
pub use gfx_backend_dx12 as dx12;

#[cfg(feature = "gfx-backend-metal")]
pub use gfx_backend_metal as metal;

#[cfg(feature = "gfx-backend-vulkan")]
pub use gfx_backend_vulkan as vulkan;

