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
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]

extern crate ash;

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate failure;
extern crate veclist;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

extern crate hibitset;
extern crate relevant;
extern crate smallvec;

#[cfg(test)]
extern crate rand;

mod block;
mod error;
mod heaps;
mod impls;
mod mapping;
mod memory;
mod util;

pub mod allocator;
pub mod usage;

pub use block::Block;
pub use error::{AllocationError, MappingError, MemoryError, OutOfMemoryError};
pub use heaps::{Config, Heaps, MemoryBlock};
pub use mapping::{write::Write, Coherent, MappedRange, MaybeCoherent, NonCoherent};
pub use memory::Memory;
pub use usage::Usage;

