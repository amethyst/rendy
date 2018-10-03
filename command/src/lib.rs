//! Crate level docs.

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
// TODO: Cleanup the code and remove.
#![allow(dead_code, unreachable_code, unused_variables)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
extern crate relevant;

extern crate rendy_chain as chain;
extern crate rendy_resource as resource;

#[cfg(feature = "hal")]
extern crate gfx_hal as hal;

#[cfg(feature = "ash")]
extern crate ash;

mod impls;

pub mod device;
pub mod error;
pub mod frame;

pub mod buffer;
pub mod capability;
pub mod encoder;
pub mod family;
pub mod pool;
pub mod queue;

pub use device::{CommandBuffer, Device};
