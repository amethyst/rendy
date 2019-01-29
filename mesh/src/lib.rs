//!
//! This crates provides means to deal with vertex buffers and meshes.
//!
//! `Attribute` and `VertexFormat` allow vertex structure to declare semantics.
//! `Mesh` can be created from typed vertex structures and provides mechanism to bind
//! vertex attributes required by shader interface.
//!

#![forbid(overflowing_literals)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(intra_doc_link_resolution_failure)]
#![warn(path_statements)]
#![warn(trivial_bounds)]
#![warn(type_alias_bounds)]
#![warn(unconditional_recursion)]
#![warn(unions_with_drop_fields)]
#![warn(while_true)]
#![warn(unused)]
#![warn(bad_style)]
#![warn(future_incompatible)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]
#![allow(unused_unsafe)]

use rendy_command as command;
use rendy_factory as factory;
use rendy_resource as resource;
use rendy_util as util;

mod format;
mod mesh;
mod vertex;

pub use crate::{
    mesh::*,
    vertex::*,
    format::*,
};
