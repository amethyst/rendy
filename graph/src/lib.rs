//! Framegraph implementation for Rendy engine.

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
use rendy_chain as chain;
use rendy_command as command;
use rendy_core as core;
use rendy_factory as factory;
use rendy_frame as frame;
use rendy_memory as memory;
use rendy_resource as resource;
use rendy_wsi as wsi;

/// Id of the buffer in graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(usize);

/// Id of the image (or target) in graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageId(usize);

/// Id of the node in graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

mod graph;
mod node;

pub use self::{graph::*, node::*};
