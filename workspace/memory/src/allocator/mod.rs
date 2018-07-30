
pub mod arena;
pub mod dedicated;
pub mod chunk;

use block::Block;
use memory::{Device, MemoryError};

/// Allocator trait implemented for various allocators.
pub trait Allocator<T> {

    /// Block type returned by allocator.
    type Block: Block<T>;

    /// Allocate block of memory.
    /// On success returns allocated block and amount of memory consumed from device.
    fn alloc<D>(&mut self, device: &D, size: u64, align: u64) -> Result<(Self::Block, u64), MemoryError>
    where
        D: Device<T>,
    ;

    /// Free block of memory.
    /// Returns amount of memory returned to the device.
    fn free<D>(&mut self, device: &D, block: Self::Block) -> u64
    where
        D: Device<T>,
    ;
}
