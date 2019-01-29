//! This crate can derive synchronization required
//! for the dependency chain of the whole execution graph.

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

/// Unique resource id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub usize);

mod chain;
mod collect;
mod node;
mod resource;
mod schedule;
mod sync;

pub use crate::{
    chain::{Chain, Link, LinkNode},
    collect::{collect, Chains, Unsynchronized},
    node::{Node, State, BufferState, ImageState},
    resource::{AccessFlags, Buffer, Image, Resource, UsageFlags},
    schedule::{Family, Queue, QueueId, Schedule, Submission, SubmissionId},
    sync::{sync, SyncData, Barrier, Barriers, BufferBarriers, ImageBarriers, Guard, Wait, Signal},
};
