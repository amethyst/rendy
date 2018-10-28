//! Buffer usage, creation-info and wrappers.

mod usage;

use ash::vk::{Buffer as AshBuffer, BufferCreateInfo};

pub use self::usage::*;
use memory::{Block, MemoryBlock};
use relevant::Relevant;

use escape::Escape;

/// Generic buffer object wrapper.
///
/// # Parameters
///
/// `T` - type of the memory object of memory block.
/// `B` - raw buffer type.
#[derive(Debug)]
pub struct Buffer {
    pub(crate) inner: Escape<Inner>,
    pub(crate) info: BufferCreateInfo,
}

impl Buffer {
    /// Get buffers memory block.
    pub fn block(&self) -> &impl Block {
        &self.inner.block
    }

    /// Get buffers memory block.
    pub fn block_mut(&mut self) -> &mut impl Block {
        &mut self.inner.block
    }

    /// Get raw buffer handle.
    pub unsafe fn raw(&self) -> AshBuffer {
        self.inner.raw
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) block: MemoryBlock,
    pub(crate) raw: AshBuffer,
    pub(crate) relevant: Relevant,
}
