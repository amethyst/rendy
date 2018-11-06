//! Buffer usage, creation-info and wrappers.

mod usage;

pub use self::usage::*;
use memory::{Block, MemoryBlock};

use crate::escape::Escape;

/// Buffer info.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    /// Buffer memory alignment.
    pub align: u64,

    /// Buffer size.
    pub size: u64,

    /// Buffer usage flags.
    pub usage: gfx_hal::buffer::Usage,
}

/// Generic buffer object wrapper.
///
/// # Parameters
///
/// `T` - type of the memory object of memory block.
/// `B` - raw buffer type.
#[derive(Debug)]
pub struct Buffer<B: gfx_hal::Backend> {
    pub(crate) inner: Escape<Inner<B>>,
    pub(crate) info: Info,
}

#[derive(Debug)]
pub(crate) struct Inner<B: gfx_hal::Backend> {
    pub(crate) block: MemoryBlock<B>,
    pub(crate) raw: B::Buffer,
    pub(crate) relevant: relevant::Relevant,
}

impl<B> Buffer<B>
where
    B: gfx_hal::Backend,
{
    /// Get buffers memory block.
    pub fn block(&self) -> &impl Block<B> {
        &self.inner.block
    }

    /// Get buffers memory block.
    pub fn block_mut(&mut self) -> &mut impl Block<B> {
        &mut self.inner.block
    }

    /// Get raw buffer handle.
    ///
    /// # Safety
    ///
    /// Raw buffer handler should not be usage to violate this object valid usage.
    pub unsafe fn raw(&self) -> &B::Buffer {
        &self.inner.raw
    }
}
