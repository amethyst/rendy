//! GPU memory management

#![deny(unused_must_use)]

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate failure;
extern crate veclist;
extern crate either;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

extern crate hibitset;

mod block;
mod device;
mod error;
mod heaps;
mod util;
mod impls;

pub mod allocator;
pub mod memory;
pub mod usage;
pub mod mapping;

pub use block::Block;
pub use device::Device;
pub use error::{AllocationError, OutOfMemoryError, MappingError, MemoryError};
pub use heaps::{Heaps, SmartBlock, HeapsConfig};
pub use usage::Usage;

#[cfg(feature = "gfx-hal")]
extern crate gfx_hal as hal;

#[cfg(feature = "ash")]
extern crate ash;

#[cfg(feature = "ash")]
extern crate smallvec;