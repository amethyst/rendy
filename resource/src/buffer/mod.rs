//! Buffer usage, creation-info and wrappers.

mod usage;

use ash::vk;

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
    pub(crate) info: vk::BufferCreateInfo,
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) block: MemoryBlock,
    pub(crate) raw: vk::Buffer,
    pub(crate) relevant: Relevant,
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
    /// 
    /// # Safety
    /// 
    /// Raw buffer handler should not be usage to violate this object valid usage.
    pub unsafe fn raw(&self) -> vk::Buffer {
        self.inner.raw
    }
}
