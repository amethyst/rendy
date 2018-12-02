//! Buffer usage, creation-info and wrappers.

mod usage;

pub use self::usage::*;
use memory::{Block, MemoryBlock};

use crate::escape::{Escape, KeepAlive};

/// Buffer info.
#[derive(Clone, Copy, Debug)]
pub struct Info {
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
    pub(crate) escape: Escape<Inner<B>>,
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
    /// Creates [`KeepAlive`] handler to extend buffer lifetime.
    /// 
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        Escape::keep_alive(&self.escape)
    }

    /// Get buffers memory [`Block`].
    /// 
    /// [`Block`]: ../memory/trait.Block.html
    pub fn block(&self) -> &impl Block<B> {
        &self.escape.block
    }

    /// Get buffers memory [`Block`].
    /// 
    /// [`Block`]: ../memory/trait.Block.html
    pub fn block_mut(&mut self) -> &mut impl Block<B> {
        &mut self.escape.block
    }

    /// Get raw buffer handle.
    ///
    /// # Safety
    ///
    /// Raw buffer handler should not be usage to violate this object valid usage.
    pub fn raw(&self) -> &B::Buffer {
        &self.escape.raw
    }

    /// Get buffer info.
    pub fn info(&self) -> &Info {
        &self.info
    }

    /// Get buffer info.
    pub fn size(&self) -> u64 {
        self.info.size
    }
}
