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
    escape: Escape<Inner<B>>,
    info: Info,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Inner<B: gfx_hal::Backend> {
    block: Option<MemoryBlock<B>>,
    raw: B::Image,
    relevant: relevant::Relevant,
}

impl<B> Inner<B>
where
    B: gfx_hal::Backend,
{
    #[doc(hidden)]
    pub fn dispose(self) -> (B::Image, Option<MemoryBlock<B>>) {
        self.relevant.dispose();
        (self.raw, self.block)
    }
}

impl<B> Image<B>
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
    /// `info` must match information about raw image.
    /// `block` if provided must be the one bound to the raw image.
    /// `terminal` will receive image and memory block upon drop, it must free image and memory properly.
    ///
    #[doc(hidden)]
    pub unsafe fn new(
        info: Info,
        raw: B::Image,
        block: Option<MemoryBlock<B>>,
        terminal: &Terminal<Inner<B>>,
    ) -> Self {
        Image {
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
    escape: Escape<InnerView<B>>,
    info: ViewInfo,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct InnerView<B: gfx_hal::Backend> {
    raw: B::ImageView,
    image_kp: KeepAlive,
    relevant: relevant::Relevant,
}

impl<B> InnerView<B>
where
    B: gfx_hal::Backend,
{
    #[doc(hidden)]
    pub fn dispose(self) -> (B::ImageView, KeepAlive) {
        self.relevant.dispose();
        (self.raw, self.image_kp)
    }
}

impl<B> ImageView<B>
where
    B: gfx_hal::Backend,
{
    #[doc(hidden)]
    pub unsafe fn new(
        info: ViewInfo,
        image: &Image<B>,
        raw: B::ImageView,
        terminal: &Terminal<InnerView<B>>,
    ) -> Self {
        ImageView {
            escape: terminal.escape(InnerView {
                raw,
                image_kp: image.keep_alive(),
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
    pub fn unescape(self) -> Option<InnerView<B>> {
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
    /// Raw image handler should not be usage to violate this object valid usage.
    pub fn raw(&self) -> &B::ImageView {
        &self.escape.raw
    }
}
