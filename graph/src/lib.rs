//! Framegraph implementation for Rendy engine.

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
// #![deny(unused)]
#![deny(bad_style)]
#![deny(future_incompatible)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]

extern crate ash;
extern crate rendy_chain as chain;
extern crate rendy_command as command;
extern crate rendy_resource as resource;
extern crate smallvec;

mod node;
mod graph;

pub use node::{Node, NodeDesc, NodeBuilder};
pub use graph::Graph;
