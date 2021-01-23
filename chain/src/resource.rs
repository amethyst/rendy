use rendy_core::hal;
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

impl AccessFlags for hal::buffer::Access {
    #[inline]
    fn empty() -> Self {
        Self::empty()
    }

    #[inline]
    fn exclusive(&self) -> bool {
        self.intersects(
            Self::SHADER_WRITE | Self::TRANSFER_WRITE | Self::HOST_WRITE | Self::MEMORY_WRITE,
        )
    }
}

impl AccessFlags for hal::image::Access {
    #[inline]
    fn empty() -> Self {
        Self::empty()
    }

    #[inline]
    fn exclusive(&self) -> bool {
        self.intersects(
            Self::SHADER_WRITE
                | Self::COLOR_ATTACHMENT_WRITE
                | Self::DEPTH_STENCIL_ATTACHMENT_WRITE
                | Self::TRANSFER_WRITE
                | Self::HOST_WRITE
                | Self::MEMORY_WRITE,
        )
    }
}

/// Trait to abstract of specific usage flags.
pub trait UsageFlags: Copy + Debug + BitOr<Output = Self> + BitOrAssign + 'static {}

impl UsageFlags for hal::buffer::Usage {}
impl UsageFlags for hal::image::Usage {}

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
    type Access = hal::buffer::Access;
    type Usage = hal::buffer::Usage;
    type Layout = ();

    fn no_usage() -> Self::Usage {
        hal::buffer::Usage::empty()
    }

    fn layout_for(_access: hal::buffer::Access) {}
}

/// Image resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Image;

impl Resource for Image {
    type Access = hal::image::Access;

    type Usage = hal::image::Usage;

    type Layout = hal::image::Layout;

    fn no_usage() -> Self::Usage {
        hal::image::Usage::empty()
    }

    fn layout_for(access: hal::image::Access) -> hal::image::Layout {
        let mut acc = None;
        if access.contains(hal::image::Access::INPUT_ATTACHMENT_READ) {
            acc = Some(common_layout(
                acc,
                hal::image::Layout::ShaderReadOnlyOptimal,
            ));
        }
        if access.contains(hal::image::Access::COLOR_ATTACHMENT_READ) {
            acc = Some(common_layout(
                acc,
                hal::image::Layout::ColorAttachmentOptimal,
            ));
        }
        if access.contains(hal::image::Access::COLOR_ATTACHMENT_WRITE) {
            acc = Some(common_layout(
                acc,
                hal::image::Layout::ColorAttachmentOptimal,
            ));
        }
        if access.contains(hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ) {
            acc = Some(common_layout(
                acc,
                hal::image::Layout::DepthStencilReadOnlyOptimal,
            ));
        }
        if access.contains(hal::image::Access::DEPTH_STENCIL_ATTACHMENT_WRITE) {
            acc = Some(common_layout(
                acc,
                hal::image::Layout::DepthStencilAttachmentOptimal,
            ));
        }
        if access.contains(hal::image::Access::TRANSFER_READ) {
            acc = Some(common_layout(acc, hal::image::Layout::TransferSrcOptimal));
        }
        if access.contains(hal::image::Access::TRANSFER_WRITE) {
            acc = Some(common_layout(acc, hal::image::Layout::TransferDstOptimal));
        }
        acc.unwrap_or(hal::image::Layout::General)
    }
}

fn common_layout(
    acc: Option<hal::image::Layout>,
    layout: hal::image::Layout,
) -> hal::image::Layout {
    match (acc, layout) {
        (None, layout) => layout,
        (Some(left), right) if left == right => left,
        (
            Some(hal::image::Layout::DepthStencilReadOnlyOptimal),
            hal::image::Layout::DepthStencilAttachmentOptimal,
        ) => hal::image::Layout::DepthStencilAttachmentOptimal,
        (
            Some(hal::image::Layout::DepthStencilAttachmentOptimal),
            hal::image::Layout::DepthStencilReadOnlyOptimal,
        ) => hal::image::Layout::DepthStencilAttachmentOptimal,
        (Some(_), _) => hal::image::Layout::General,
    }
}
