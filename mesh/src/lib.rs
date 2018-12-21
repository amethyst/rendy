//!
//! This crates provides means to deal with vertex buffers and meshes.
//!
//! `Attribute` and `VertexFormat` allow vertex structure to declare semantics.
//! `Mesh` can be created from typed vertex structures and provides mechanism to bind
//! vertex attributes required by shader interface.
//!

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

use rendy_factory as factory;
use rendy_resource as resource;
use rendy_util as util;

mod format;
mod mesh;
mod vertex;

pub use crate::{
    mesh::{Bind, Incompatible, IndexBuffer, Indices, Mesh, MeshBuilder, VertexBuffer},
    vertex::{
        AsAttribute, AsVertex, Attribute, Color, Normal, PosColor, PosNorm, PosNormTangTex, PosNormTex,
        PosTex, Position, Query, Tangent, TexCoord, VertexFormat, WithAttribute,
    },
    format::*,
};
