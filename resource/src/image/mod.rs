//! Image usage, format, kind, extent, creation-info and wrappers.

mod usage;

pub use {
    self::usage::{Usage, *},
    gfx_hal::image::*,
};

use crate::{
    escape::{Escape, KeepAlive, Terminal},
    memory::MemoryBlock,
};

/// Image info.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    /// Kind of the image.
    pub kind: Kind,

    /// Image mip-level count.
    pub levels: Level,

    /// Image format.
    pub format: gfx_hal::format::Format,

    /// Image tiling mode.
    pub tiling: Tiling,

    /// Image view capabilities.
    pub view_caps: ViewCapabilities,

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
    escape: Escape<(B::Image, Option<MemoryBlock<B>>)>,
    info: Info,
}

impl<B> Image<B>
where
    B: gfx_hal::Backend,
{
    /// Wrap an image.
    ///
    /// # Safety
    ///
    /// `info` must match information about raw image.
    /// `block` if provided must be the one bound to the raw image.
    /// `terminal` will receive image and memory block upon drop, it must free image and memory properly.
    ///
    pub unsafe fn new(
        info: Info,
        raw: B::Image,
        block: Option<MemoryBlock<B>>,
        terminal: &Terminal<(B::Image, Option<MemoryBlock<B>>)>,
    ) -> Self {
        Image {
            escape: terminal.escape((raw, block)),
            info,
        }
    }

    /// This will return `None` and would be equivalent to dropping
    /// if there are `KeepAlive` created from this `Image.
    pub fn unescape(self) -> Option<(B::Image, Option<MemoryBlock<B>>)> {
        Escape::unescape(self.escape)
    }

    /// Creates [`KeepAlive`] handler to extend image lifetime.
    ///
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        Escape::keep_alive(&self.escape)
    }

    /// Get raw image handle.
    ///
    /// # Safety
    ///
    /// Raw image handler should not be used to violate this object valid usage.
    pub fn raw(&self) -> &B::Image {
        &self.escape.0
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

// Image view info
#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct ViewInfo {
    pub view_kind: gfx_hal::image::ViewKind,
    pub format: gfx_hal::format::Format,
    pub swizzle: gfx_hal::format::Swizzle,
    pub range: gfx_hal::image::SubresourceRange,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ImageView<B: gfx_hal::Backend> {
    escape: Escape<(B::ImageView, KeepAlive)>,
    info: ViewInfo,
}

impl<B> ImageView<B>
where
    B: gfx_hal::Backend,
{
    /// Wrap an image view.
    ///
    /// `raw` image view must be created from the `image`.
    /// `info` must match information about raw image view.
    /// `terminal` will receive image and memory block upon drop, it must free image and memory properly.
    ///
    pub unsafe fn new(
        info: ViewInfo,
        image: &Image<B>,
        raw: B::ImageView,
        terminal: &Terminal<(B::ImageView, KeepAlive)>,
    ) -> Self {
        ImageView {
            escape: terminal.escape((raw, image.keep_alive())),
            info,
        }
    }

    /// This will return `None` and would be equivalent to dropping
    /// if there are `KeepAlive` created from this `ImageView.
    pub fn unescape(self) -> Option<(B::ImageView, KeepAlive)> {
        Escape::unescape(self.escape)
    }

    /// Creates [`KeepAlive`] handler to extend image lifetime.
    ///
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        Escape::keep_alive(&self.escape)
    }

    /// Get raw image handle.
    ///
    /// # Safety
    ///
    /// Raw image handler should not be used to violate this object valid usage.
    pub fn raw(&self) -> &B::ImageView {
        &self.escape.0
    }
}
