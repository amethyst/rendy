
use std::any::Any;

pub mod arena;
pub mod dedicated;
pub mod dynamic;
// pub mod chunk;

use block::Block;
use device::Device;
use error::MemoryError;
use memory::Properties;

/// Allocator trait implemented for various allocators.
pub trait Allocator {

    /// Memory type.
    type Memory: Any;

    /// Block type returned by allocator.
    type Block: Block<Memory = Self::Memory>;

    /// Allocate block of memory.
    /// On success returns allocated block and amount of memory consumed from device.
    fn alloc<D>(&mut self, device: &D, size: u64, align: u64) -> Result<(Self::Block, u64), MemoryError>
    where
        D: Device<Memory = Self::Memory>,
    ;

    /// Free block of memory.
    /// Returns amount of memory returned to the device.
    fn free<D>(&mut self, device: &D, block: Self::Block) -> u64
    where
        D: Device<Memory = Self::Memory>,
    ;
}
