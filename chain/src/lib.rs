
#[macro_use]
extern crate bitflags;

extern crate fnv;

extern crate rendy_resource;


/// Unique resource id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u64);

pub mod access;
pub mod chain;
pub mod node;
pub mod resource;
pub mod stage;
pub mod schedule;
pub mod collect;
pub mod sync;
