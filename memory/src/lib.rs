//! GPU memory management

#![deny(unused_must_use)]

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate failure;
extern crate veclist;
extern crate either;

mod allocator;
mod block;
mod device;
mod error;
mod heaps;
mod memory;
mod usage;
mod util;
mod map;


pub use allocator::{
    Allocator,
    dedicated::{DedicatedAllocator, DedicatedBlock},
    arena::{ArenaAllocator, ArenaBlock},
    chunk::{ChunkAllocator, ChunkBlock},
    dynamic::{DynamicAllocator, DynamicBlock},
};
pub use block::Block;
pub use device::Device;
pub use error::{OutOfMemoryError, MappingError, MemoryError};
pub use heaps::{Heaps, HeapsBlock};
pub use memory::{Memory, Properties};
pub use usage::{Usage, Value, Data, Dynamic, Upload, Download};

#[cfg(feature = "gfx-hal")]
extern crate gfx_hal as hal;

#[cfg(feature = "gfx-hal")]
mod hal_impls;

#[cfg(feature = "ash")]
extern crate ash;

#[cfg(feature = "ash")]
extern crate smallvec;

#[cfg(feature = "ash")]
mod ash_impls;
