//! This crate can derive synchronization required
//! for the dependency chain of the whole execution graph.

#[warn(missing_debug_implementations,
       missing_copy_implementations,
       missing_docs,
       trivial_casts,
       trivial_numeric_casts,
       unused_extern_crates,
       unused_import_braces,
       unused_qualifications)]

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
