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
    sync::{sync, SyncData, Barriers, BufferBarriers, ImageBarriers, Guard, Wait, Signal},
};
