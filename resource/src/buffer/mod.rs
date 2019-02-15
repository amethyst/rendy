//! Buffer usage, creation-info and wrappers.

mod usage;

pub use {
    self::usage::{Usage, *},
    gfx_hal::buffer::*,
};

use crate::{
    escape::{Escape, KeepAlive, Terminal},
    memory::{Block, MappedRange, MemoryBlock},
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
#[derive(Debug)]
pub struct Buffer<B: gfx_hal::Backend> {
    escape: Escape<(B::Buffer, MemoryBlock<B>)>,
    info: Info,
}

impl<B> Buffer<B>
where
    B: gfx_hal::Backend,
{
    /// Wrap a buffer.
    ///
    /// # Safety
    ///
    /// `info` must match information about raw buffer.
    /// `block` if provided must be the one bound to the raw buffer.
    /// `terminal` will receive buffer and memory block upon drop, it must free buffer and memory properly.
    ///
    pub unsafe fn new(
        info: Info,
        raw: B::Buffer,
        block: MemoryBlock<B>,
        terminal: &Terminal<(B::Buffer, MemoryBlock<B>)>,
    ) -> Self {
        Buffer {
            escape: terminal.escape((raw, block)),
            info,
        }
    }

    /// This will return `None` and would be equivalent to dropping
    /// if there are `KeepAlive` created from this `Buffer.
    pub fn unescape(self) -> Option<(B::Buffer, MemoryBlock<B>)> {
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
        self.escape
            .1
            .properties()
            .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
    }

    /// Map range of the buffer to the CPU accessible memory.
    pub fn map<'a>(
        &'a mut self,
        device: &impl gfx_hal::Device<B>,
        range: std::ops::Range<u64>,
    ) -> Result<MappedRange<'a, B>, gfx_hal::mapping::Error> {
        self.escape.1.map(device, range)
    }

    // /// Map range of the buffer to the CPU accessible memory.
    // pub fn persistent_map(&mut self, range: std::ops::Range<u64>) -> Result<MappedRange<'a, B>, gfx_hal::mapping::Error> {
    //     self.escape.1.map(device, range)
    // }

    /// Get raw buffer handle.
    ///
    /// # Safety
    ///
    /// Raw buffer handler should not be usage to violate this object valid usage.
    pub fn raw(&self) -> &B::Buffer {
        &self.escape.0
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
