//! This crates provides means to deal with vertex buffers and meshes.
//! `Attribute` and `VertexFormat` allow vertex structure to declare semantics.
//! `Mesh` can be created from typed vertex structures and provides mechanism to bind
//! vertex attributes required by shader interface.

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
use rendy_command as command;
use rendy_core as core;
use rendy_factory as factory;
use rendy_memory as memory;
use rendy_resource as resource;

mod format;
mod mesh;

pub use crate::{format::*, mesh::*};
pub use rendy_core::types::vertex::*;
