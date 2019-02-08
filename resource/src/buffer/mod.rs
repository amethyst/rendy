//! Buffer usage, creation-info and wrappers.

mod usage;

pub use {
    gfx_hal::buffer::*,
    self::usage::{*, Usage},
};

use crate::{
    escape::{Escape, KeepAlive, Terminal},
    memory::{Block, MemoryBlock, MappedRange},
};

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
    escape: Escape<Inner<B>>,
    info: Info,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Inner<B: gfx_hal::Backend> {
    block: MemoryBlock<B>,
    raw: B::Buffer,
    relevant: relevant::Relevant,
}

impl<B> Inner<B>
where
    B: gfx_hal::Backend,
{
    pub(super) fn dispose(self) -> (B::Buffer, MemoryBlock<B>) {
        self.relevant.dispose();
        (self.raw, self.block)
    }
}

impl<B> Buffer<B>
where
    B: gfx_hal::Backend,
{
    /// # Disclaimer
    /// 
    /// This function is designed to use by other rendy crates.
    /// User experienced enough to use it properly can find it without documentation.
    /// 
    /// # Safety
    /// 
    /// `info` must match information about raw buffer.
    /// `block` if provided must be the one bound to the raw buffer.
    /// `terminal` will receive buffer and memory block upon drop, it must free buffer and memory properly.
    /// 
    #[doc(hidden)]
    pub unsafe fn new(info: Info, raw: B::Buffer, block: MemoryBlock<B>, terminal: &Terminal<Inner<B>>) -> Self {
        Buffer {
            escape: terminal.escape(Inner {
                block,
                raw,
                relevant: relevant::Relevant,
            }),
            info,
        }
    }

    /// # Disclaimer
    /// 
    /// This function is designed to use by other rendy crates.
    /// User experienced enough to use it properly can find it without documentation.
    #[doc(hidden)]
    pub fn unescape(self) -> Option<Inner<B>> {
        Escape::unescape(self.escape)
    }

    /// Creates [`KeepAlive`] handler to extend buffer lifetime.
    /// 
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        Escape::keep_alive(&self.escape)
    }

    /// Check if this buffer could is bound to CPU visible memory and therefore mappable.
    /// If this function returns `false` `map` will always return `InvalidAccess`.
    /// 
    /// [`map`]: #method.map
    /// [`InvalidAccess`]: https://docs.rs/gfx-hal/0.1/gfx_hal/mapping/enum.Error.html#InvalidAccess
    pub fn visible(&self) -> bool {
        self.escape.block.properties().contains(gfx_hal::memory::Properties::CPU_VISIBLE)
    }

    /// Map range of the buffer to the CPU accessible memory.
    pub fn map<'a>(&'a mut self, device: &B::Device, range: std::ops::Range<u64>) -> Result<MappedRange<'a, B>, gfx_hal::mapping::Error> {
        self.escape.block.map(device, range)
    }

    // /// Map range of the buffer to the CPU accessible memory.
    // pub fn persistent_map(&mut self, range: std::ops::Range<u64>) -> Result<MappedRange<'a, B>, gfx_hal::mapping::Error> {
    //     self.escape.block.map(device, range)
    // }

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
