//! GPU memory management
//!
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
#![allow(unused_unsafe)]

mod allocator;
mod block;
mod heaps;
mod mapping;
mod memory;
mod util;
mod usage;

pub use crate::{
    allocator::*,
    block::Block,
    heaps::{Heaps, HeapsConfig, MemoryBlock, HeapsError},
    mapping::{write::Write, Coherent, MappedRange, MaybeCoherent, NonCoherent},
    memory::Memory,
    usage::*,
};
