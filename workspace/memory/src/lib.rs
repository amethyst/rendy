//! GPU memory management

extern crate gfx_hal as hal;

mod allocator;
mod block;
mod dedicated;
mod device;
mod memory;
mod sub;

pub use allocator::Allocator;
pub use block::Block;
pub use dedicated::{DedicatedAllocator, DedicatedBlock};
pub use device::{DeviceMemory, DeviceMemoryBlock};
pub use memory::Memory;
pub use sub::SubAllocator;

// #[derive(Copy, Clone, Debug)]
// pub enum Usage {
//     /// Full speed GPU access. Optimal for render targets and resourced memory.
//     Data,
//     /// CPU to GPU data flow with update commands. Used for dynamic buffer data, typically constant buffers.
//     Dynamic,
//     /// CPU to GPU data flow with mapping. Used for staging for upload to GPU.
//     Upload,
//     /// GPU to CPU data flow with mapping. Used for staging for download from GPU.
//     Download,
// }

/// Replace with something better.
pub mod default {

pub type Memory = ::DeviceMemory<::DedicatedAllocator>;
pub type Block<B> = ::DeviceMemoryBlock<::DedicatedBlock<<B as ::hal::Backend>::Memory>>;

}