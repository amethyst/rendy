//! GPU memory management

extern crate gfx_hal as hal;

#[macro_use]
extern crate failure;

mod allocator;
mod block;
mod device;
mod memory;
mod sub;
mod usage;

pub use allocator::Allocator;
pub use block::{Block, MappingError};
pub use device::{DeviceMemory, DeviceMemoryBlock};
pub use memory::Memory;
pub use sub::{SubAllocator, dedicated::{DedicatedAllocator, DedicatedBlock}};
pub use usage::{Usage, Value, Data, Dynamic, Upload, Download};


/// Replace with something better.
pub mod default {

pub type Memory = ::DeviceMemory<::DedicatedAllocator>;
pub type Block<B> = ::DeviceMemoryBlock<::DedicatedBlock<<B as ::hal::Backend>::Memory>>;

}
