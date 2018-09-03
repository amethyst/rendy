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

#![warn(unreachable_pub)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]


// TODO: Cleanup the code and remove.
#![allow(dead_code, unreachable_code, unused_variables)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
extern crate relevant;

extern crate rendy_resource as resource;
extern crate rendy_chain as chain;

pub mod error;
pub mod frame;
pub mod device;

pub mod capability;
pub mod buffer;
pub mod encoder;
pub mod pool;
pub mod queue;
pub mod family;

pub use device::{CommandBuffer, Device};
