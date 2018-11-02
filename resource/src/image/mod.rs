//! Image usage, format, kind, extent, creation-info and wrappers.

mod usage;

pub use self::usage::*;

use ash::vk;

use memory::{Block, MemoryBlock};
use relevant::Relevant;

use escape::Escape;

/// Generic image object wrapper.
///
/// # Parameters
///
/// `T` - type of the memory object of memory block.
/// `B` - raw image type.
#[derive(Debug)]
pub struct Image {
    pub(super) inner: Escape<Inner>,
    pub(super) info: vk::ImageCreateInfo,
}

#[derive(Debug)]
pub(super) struct Inner {
    pub(super) block: MemoryBlock,
    pub(super) raw: vk::Image,
    pub(super) relevant: Relevant,
}

impl Image {
    /// Get buffers memory block.
    pub fn block(&self) -> &impl Block {
        &self.inner.block
    }

    /// Get buffers memory block.
    pub fn block_mut(&mut self) -> &mut impl Block {
        &mut self.inner.block
    }

    /// Get raw image handle.
    ///
    /// # Safety
    ///
    /// Raw image handler should not be usage to violate this object valid usage.
    pub unsafe fn raw(&self) -> vk::Image {
        self.inner.raw
    }

    /// Get extent of the image.
    pub fn extent(&self) -> vk::Extent3D {
        self.info.extent
    }
}
