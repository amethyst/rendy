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

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate failure;
extern crate veclist;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

extern crate hibitset;

mod block;
mod device;
mod error;
mod heaps;
mod memory;
mod util;
mod mapping;
mod impls;

pub mod allocator;
pub mod usage;

pub use block::Block;
pub use device::Device;
pub use error::{AllocationError, OutOfMemoryError, MappingError, MemoryError};
pub use heaps::{Heaps, MemoryBlock, HeapsConfig};
pub use mapping::{MappedRange, write::Write, NonCoherent, Coherent, MaybeCoherent};
pub use memory::{Properties, Memory};
pub use usage::Usage;

#[cfg(feature = "gfx-hal")]
extern crate gfx_hal as hal;

#[cfg(feature = "ash")]
extern crate ash;

#[cfg(feature = "ash")]
extern crate smallvec;