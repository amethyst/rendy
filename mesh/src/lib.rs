//!
//! This crates provides means to deal with vertex buffers and meshes.
//!
//! `Attribute` and `VertexFormat` allow vertex structure to declare semantics.
//! `Mesh` can be created from typed vertex structures and provides mechanism to bind
//! vertex attributes required by shader interface.
//!

extern crate ash;
extern crate failure;
extern crate rendy_command as command;
extern crate rendy_factory as factory;
extern crate rendy_memory as memory;
extern crate rendy_resource as resource;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

extern crate smallvec;

mod mesh;
mod utils;
mod vertex;

pub use mesh::{Bind, Incompatible, IndexBuffer, Indices, Mesh, MeshBuilder, VertexBuffer};
pub use vertex::{
    AsAttribute, AsVertex, Attribute, Color, Normal, PosColor, PosNorm, PosNormTangTex, PosNormTex,
    PosTex, Position, Query, Tangent, TexCoord, VertexFormat, WithAttribute,
};
