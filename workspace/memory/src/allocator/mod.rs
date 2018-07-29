
pub mod arena;
pub mod dedicated;

use block::Block;
use memory::{Device, MemoryError};

/// Allocator trait implemented for various allocators.
pub trait Allocator<T> {
    /// Block type returned by allocator.
    type Block: Block<T>;

    fn alloc<D>(&mut self, device: &D, size: u64, align: u64) -> Result<(Self::Block, u64), MemoryError>
    where
        D: Device<T>,
    ;

    fn free<D>(&mut self, device: &D, block: Self::Block) -> u64
    where
        D: Device<T>,
    ;
}
