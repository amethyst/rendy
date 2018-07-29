//! GPU memory management

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate failure;

#[cfg(feature = "gfx-hal")]
extern crate gfx_hal as hal;

mod allocator;
mod block;
mod heaps;
mod memory;
mod usage;
mod util;

pub use block::Block;
pub use heaps::{Heaps, HeapsBlock};
pub use memory::{Memory, Device, MemoryError, MappingError, Properties};
pub use allocator::{Allocator, dedicated::{DedicatedAllocator, DedicatedBlock}, arena::{ArenaAllocator, ArenaBlock}};
pub use usage::{Usage, Value, Data, Dynamic, Upload, Download};

