
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

pub extern crate rendy_command as command;
pub extern crate rendy_factory as factory;
pub extern crate rendy_frame as frame;
pub extern crate rendy_graph as graph;
pub extern crate rendy_memory as memory;
pub extern crate rendy_mesh as mesh;
pub extern crate rendy_resource as resource;
pub extern crate rendy_shader as shader;
pub extern crate rendy_wsi as wsi;

pub extern crate gfx_hal as hal;

#[cfg(feature = "gfx-backend-empty")]
pub extern crate gfx_backend_empty as empty;

#[cfg(feature = "gfx-backend-dx12")]
pub extern crate gfx_backend_dx12 as dx12;

#[cfg(feature = "gfx-backend-metal")]
pub extern crate gfx_backend_metal as metal;

#[cfg(feature = "gfx-backend-vulkan")]
pub extern crate gfx_backend_vulkan as vulkan;
