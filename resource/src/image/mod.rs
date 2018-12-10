//! Image usage, format, kind, extent, creation-info and wrappers.

mod usage;

pub use self::usage::*;

use crate::{
    escape::{Escape, KeepAlive},
    memory::{Block, MemoryBlock},
};

/// Image info.
#[derive(Clone, Copy, Debug)]
pub struct Info {
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
    pub(super) escape: Escape<Inner<B>>,
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
    /// Creates [`KeepAlive`] handler to extend image lifetime.
    /// 
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        Escape::keep_alive(&self.escape)
    }

    /// Get images memory [`Block`].
    /// 
    /// [`Block`]: ../memory/trait.Block.html
    pub fn block(&self) -> &impl Block<B> {
        &self.escape.block
    }

    /// Get images memory [`Block`].
    /// 
    /// [`Block`]: ../memory/trait.Block.html
    pub fn block_mut(&mut self) -> &mut impl Block<B> {
        &mut self.escape.block
    }

    /// Get raw image handle.
    ///
    /// # Safety
    ///
    /// Raw image handler should not be usage to violate this object valid usage.
    pub fn raw(&self) -> &B::Image {
        &self.escape.raw
    }

    /// Get image [`Info`].
    /// 
    /// [`Info`]: struct.Info.html
    pub fn info(&self) -> Info {
        self.info
    }

    /// Get [`Kind`] of the image.
    /// 
    /// [`Kind`]: ../gfx-hal/image/struct.Kind.html
    pub fn kind(&self) -> gfx_hal::image::Kind {
        self.info.kind
    }

    /// Get [`Format`] of the image.
    /// 
    /// [`Format`]: ../gfx-hal/format/struct.Format.html
    pub fn format(&self) -> gfx_hal::format::Format {
        self.info.format
    }

    /// Get levels count of the image.
    pub fn levels(&self) -> u8 {
        self.info.levels
    }

    /// Get layers count of the image.
    pub fn layers(&self) -> u16 {
        self.info.kind.num_layers()
    }
}
