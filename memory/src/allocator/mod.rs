//! This module provides `Allocator` trait and few allocators that implements the trait.

use std::{any::Any, fmt};

mod arena;
mod dedicated;
mod dynamic;
// mod chunk;

use block::Block;
use device::Device;
use error::MemoryError;
use memory::Memory;

pub use self::{
    arena::{ArenaAllocator, ArenaBlock, ArenaConfig},
    dedicated::{DedicatedAllocator, DedicatedBlock},
    dynamic::{DynamicAllocator, DynamicBlock, DynamicConfig},
};

/// Allocator trait implemented for various allocators.
pub trait Allocator {
    /// Memory type.
    type Memory: Any;

    /// Block type returned by allocator.
    type Block: Block<Memory = Self::Memory>;

    /// Allocate block of memory.
    /// On success returns allocated block and amount of memory consumed from device.
    fn alloc<D>(
        &mut self,
        device: &D,
        size: u64,
        align: u64,
    ) -> Result<(Self::Block, u64), MemoryError>
    where
        D: Device<Memory = Self::Memory>;

    /// Free block of memory.
    /// Returns amount of memory returned to the device.
    fn free<D>(&mut self, device: &D, block: Self::Block) -> u64
    where
        D: Device<Memory = Self::Memory>;
}

fn memory_ptr_fmt<T: fmt::Debug>(
    memory: &*const Memory<T>,
    fmt: &mut fmt::Formatter,
) -> Result<(), fmt::Error> {
    unsafe {
        if fmt.alternate() {
            write!(fmt, "*const {:#?}", **memory)
        } else {
            write!(fmt, "*const {:?}", **memory)
        }
    }
}
