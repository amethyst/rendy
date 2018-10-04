//! This crate can derive synchronization required
//! for the dependency chain of the whole execution graph.

// #![forbid(overflowing_literals)]
// #![deny(missing_copy_implementations)]
// #![deny(missing_debug_implementations)]
// #![deny(missing_docs)]
// #![deny(intra_doc_link_resolution_failure)]
// #![deny(path_statements)]
// #![deny(trivial_bounds)]
// #![deny(type_alias_bounds)]
// #![deny(unconditional_recursion)]
// #![deny(unions_with_drop_fields)]
// #![deny(while_true)]
// #![deny(unused)]
// #![deny(bad_style)]
// #![deny(future_incompatible)]
// #![warn(rust_2018_compatibility)]
// #![warn(rust_2018_idioms)]

#[macro_use]
extern crate bitflags;

extern crate fnv;

extern crate rendy_resource;

/// Unique resource id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u64);

/// ???
mod access;
/// ???
mod chain;
/// ???
mod collect;
/// ???
mod node;
/// ???
mod resource;
/// ???
mod schedule;
/// ???
mod stage;
/// ???
mod sync;

pub use chain::Chain;
pub use node::{Node, State};
pub use resource::{Buffer, Image, Resource};
pub use stage::{PipelineStageFlags, GraphicsPipelineStage, ComputePipelineStage};
pub use sync::SyncData;
pub use schedule::Schedule;

