use std::{
    fmt::Debug,
    ops::{BitOr, BitOrAssign},
};

/// Trait to abstract of specific access flags.
pub trait AccessFlags: Copy + Debug + BitOr<Output = Self> + BitOrAssign + 'static {
    /// Get flags value with no flags set.
    fn empty() -> Self;

    /// Check if this access must be exclusive.
    /// 
    /// Basically this checks if all flags are known read flags.
    fn exclusive(&self) -> bool;
}

impl AccessFlags for gfx_hal::buffer::Access {
    #[inline]
    fn empty() -> Self {
        Self::empty()
    }

    #[inline]
    fn exclusive(&self) -> bool {
        self.intersects(
            Self::SHADER_WRITE | Self::TRANSFER_WRITE | Self::HOST_WRITE | Self::MEMORY_WRITE
        )
    }
}

impl AccessFlags for gfx_hal::image::Access {
    #[inline]
    fn empty() -> Self {
        Self::empty()
    }

    #[inline]
    fn exclusive(&self) -> bool {
        self.intersects(
            Self::SHADER_WRITE | Self::COLOR_ATTACHMENT_WRITE | Self::DEPTH_STENCIL_ATTACHMENT_WRITE | Self::TRANSFER_WRITE | Self::HOST_WRITE | Self::MEMORY_WRITE
        )
    }
}

/// Trait to abstract of specific usage flags.
pub trait UsageFlags: Copy + Debug + BitOr<Output = Self> + BitOrAssign + 'static {}

impl UsageFlags for gfx_hal::buffer::Usage {}
impl UsageFlags for gfx_hal::image::Usage {}

/// Abstracts resource types that uses different usage flags and layouts types.
pub trait Resource: 'static {
    /// Access flags for resource type.
    type Access: AccessFlags;

    /// Usage flags type for the resource.
    type Usage: UsageFlags;

    /// Layout type for the resource.
    type Layout: Copy + Debug + 'static;

    /// Empty usage.
    fn no_usage() -> Self::Usage;

    /// Layout suitable for specified accesses.
    fn layout_for(access: Self::Access) -> Self::Layout;
}

/// Buffer resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Buffer;

impl Resource for Buffer {
    type Access = gfx_hal::buffer::Access;
    type Usage = gfx_hal::buffer::Usage;
    type Layout = ();

    fn no_usage() -> Self::Usage {
        gfx_hal::buffer::Usage::empty()
    }

    fn layout_for(_access: gfx_hal::buffer::Access) {}
}

/// Image resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Image;

impl Resource for Image {

    type Access = gfx_hal::image::Access;

    type Usage = gfx_hal::image::Usage;

    type Layout = gfx_hal::image::Layout;

    fn no_usage() -> Self::Usage {
        gfx_hal::image::Usage::empty()
    }

    fn layout_for(access: gfx_hal::image::Access) -> gfx_hal::image::Layout {
        let mut acc = None;
        if access.contains(gfx_hal::image::Access::INPUT_ATTACHMENT_READ) {
            acc = Some(common_layout(acc, gfx_hal::image::Layout::ShaderReadOnlyOptimal));
        }
        if access.contains(gfx_hal::image::Access::COLOR_ATTACHMENT_READ) {
            acc = Some(common_layout(acc, gfx_hal::image::Layout::ColorAttachmentOptimal));
        }
        if access.contains(gfx_hal::image::Access::COLOR_ATTACHMENT_WRITE) {
            acc = Some(common_layout(acc, gfx_hal::image::Layout::ColorAttachmentOptimal));
        }
        if access.contains(gfx_hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ) {
            acc = Some(common_layout(acc, gfx_hal::image::Layout::DepthStencilReadOnlyOptimal));
        }
        if access.contains(gfx_hal::image::Access::DEPTH_STENCIL_ATTACHMENT_WRITE) {
            acc = Some(common_layout(acc, gfx_hal::image::Layout::DepthStencilAttachmentOptimal));
        }
        if access.contains(gfx_hal::image::Access::TRANSFER_READ) {
            acc = Some(common_layout(acc, gfx_hal::image::Layout::TransferSrcOptimal));
        }
        if access.contains(gfx_hal::image::Access::TRANSFER_WRITE) {
            acc = Some(common_layout(acc, gfx_hal::image::Layout::TransferDstOptimal));
        }
        acc.unwrap_or(gfx_hal::image::Layout::General)
    }
}

fn common_layout(acc: Option<gfx_hal::image::Layout>, layout: gfx_hal::image::Layout) -> gfx_hal::image::Layout {
    match (acc, layout) {
        (None, layout) => layout,
        (Some(left), right) if left == right => left,
        (
            Some(gfx_hal::image::Layout::DepthStencilReadOnlyOptimal),
            gfx_hal::image::Layout::DepthStencilAttachmentOptimal,
        ) => gfx_hal::image::Layout::DepthStencilAttachmentOptimal,
        (
            Some(gfx_hal::image::Layout::DepthStencilAttachmentOptimal),
            gfx_hal::image::Layout::DepthStencilReadOnlyOptimal,
        ) => gfx_hal::image::Layout::DepthStencilAttachmentOptimal,
        (Some(_), _) => gfx_hal::image::Layout::General,
    }
}
