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
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate bitflags;

extern crate fnv;

extern crate rendy_resource;

/// Unique resource id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u64);

/// ???
pub mod access;
/// ???
pub mod chain;
/// ???
pub mod collect;
/// ???
pub mod node;
/// ???
pub mod resource;
/// ???
pub mod schedule;
/// ???
pub mod stage;
/// ???
pub mod sync;
