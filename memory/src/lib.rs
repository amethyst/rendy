//! GPU memory management
//!
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
#![allow(unused_unsafe)]

extern crate failure;

mod block;
mod heaps;
mod mapping;
mod memory;
mod util;

pub mod allocator;
pub mod usage;

pub use crate::{
    block::Block,
    heaps::{Heaps, HeapsConfig, MemoryBlock, HeapsError},
    mapping::{write::Write, Coherent, MappedRange, MaybeCoherent, NonCoherent},
    memory::Memory,
    usage::{MemoryUsage, MemoryUsageValue},
};
