//! Image usage, format, kind, extent, creation-info and wrappers.

mod usage;

pub use self::usage::*;

use memory::{Block, MemoryBlock};

use escape::Escape;

/// Image info.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    /// Image memory alignment.
    pub align: u64,

    /// Kind of the image.
    pub kind: gfx_hal::image::Kind, 

    /// Image mip-level count.
    pub levels: gfx_hal::image::Level, 

    /// Image format.
    pub format: gfx_hal::format::Format, 

    /// Image tiling mode.
    pub tiling: gfx_hal::image::Tiling, 

    /// Image view capabilities.
    pub view_caps: gfx_hal::image::ViewCapabilities,

    /// Image usage flags.
    pub usage: gfx_hal::image::Usage,
}

/// Generic image object wrapper.
///
/// # Parameters
///
/// `T` - type of the memory object of memory block.
/// `B` - raw image type.
#[derive(Debug)]
pub struct Image<B: gfx_hal::Backend> {
    pub(super) inner: Escape<Inner<B>>,
    pub(super) info: Info,
}

#[derive(Debug)]
pub(super) struct Inner<B: gfx_hal::Backend> {
    pub(super) block: MemoryBlock<B>,
    pub(super) raw: B::Image,
    pub(super) relevant: relevant::Relevant,
}

impl<B> Image<B>
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

    /// Get raw image handle.
    ///
    /// # Safety
    ///
    /// Raw image handler should not be usage to violate this object valid usage.
    pub fn raw(&self) -> &B::Image {
        &self.inner.raw
    }

    /// Get extent of the image.
    pub fn extent(&self) -> gfx_hal::image::Extent {
        self.info.kind.extent()
    }
}
