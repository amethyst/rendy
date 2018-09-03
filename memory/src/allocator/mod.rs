//! This module provides `Allocator` trait and few allocators that implements the trait.

use std::any::Any;

mod arena;
mod dedicated;
mod dynamic;
// mod chunk;

use block::Block;
use device::Device;
use error::MemoryError;

pub use self::{
    arena::{ArenaAllocator, ArenaBlock, ArenaConfig},
    dynamic::{DynamicAllocator, DynamicBlock, DynamicConfig},
    dedicated::{DedicatedAllocator, DedicatedBlock},
};

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
