//! Image usage, format, kind, extent, creation-info and wrappers.

mod usage;

pub use self::usage::*;

use ash::vk::{Image as AshImage, ImageCreateInfo};

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
    pub(super) info: ImageCreateInfo,
}

#[derive(Debug)]
pub(super) struct Inner {
    pub(super) block: MemoryBlock,
    pub(super) raw: AshImage,
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

    /// Get raw buffer handle.
    pub unsafe fn raw(&self) -> AshImage {
        self.inner.raw
    }
}

