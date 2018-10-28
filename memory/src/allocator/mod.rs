//! This module provides `Allocator` trait and few allocators that implements the trait.

use ash::version::DeviceV1_0;

mod arena;
mod dedicated;
mod dynamic;

use block::Block;
use error::MemoryError;

pub use self::{
    arena::{ArenaAllocator, ArenaBlock, ArenaConfig},
    dedicated::{DedicatedAllocator, DedicatedBlock},
    dynamic::{DynamicAllocator, DynamicBlock, DynamicConfig},
};

/// Allocator trait implemented for various allocators.
pub trait Allocator {
    /// Block type returned by allocator.
    type Block: Block;

    /// Allocate block of memory.
    /// On success returns allocated block and amount of memory consumed from device.
    fn alloc(
        &mut self,
        device: &impl DeviceV1_0,
        size: u64,
        align: u64,
    ) -> Result<(Self::Block, u64), MemoryError>;

    /// Free block of memory.
    /// Returns amount of memory returned to the device.
    fn free(&mut self, device: &impl DeviceV1_0, block: Self::Block) -> u64;
}
