//! This crate can derive synchronization required
//! for the dependency chain of the whole execution graph.

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

extern crate ash;
extern crate fnv;

/// Unique resource id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u64);

mod access;
mod chain;
mod collect;
mod node;
mod resource;
mod schedule;
mod stage;
mod sync;

pub use access::AccessFlagsExt;
pub use chain::Chain;
pub use collect::{collect, Chains, Unsynchronized};
pub use node::{Node, State, BufferState, ImageState};
pub use resource::{Buffer, Image, Resource};
pub use schedule::{Family, FamilyId, Queue, QueueId, Schedule, Submission, SubmissionId};
pub use stage::{ComputePipelineStage, GraphicsPipelineStage};
pub use sync::{sync, SyncData, Barriers, BufferBarriers, ImageBarriers, Guard, Wait, Signal};
