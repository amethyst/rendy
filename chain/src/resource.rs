use std::{
    fmt::Debug,
    ops::{BitOr, BitOrAssign},
};

use ash::vk::{AccessFlags, BufferUsageFlags, ImageUsageFlags, ImageLayout};

/// Abstracts resource types that uses different usage flags and layouts types.
pub trait Resource: 'static {
    /// Usage flags type for the resource.
    type Usage: Copy + Debug + BitOr<Output = Self::Usage> + BitOrAssign + 'static;

    /// Layout type for the resource.
    type Layout: Copy + Debug + 'static;

    /// Empty usage.
    fn no_usage() -> Self::Usage;

    /// Layout suitable for specified accesses.
    fn layout_for(access: AccessFlags) -> Self::Layout;

    /// Check if all usage flags required for access are set.
    fn valid_usage(access: AccessFlags, usage: Self::Usage) -> bool;
}

const BUFFER_ACCESSES: [AccessFlags; 8] = [
    AccessFlags::INDIRECT_COMMAND_READ,
    AccessFlags::INDEX_READ,
    AccessFlags::VERTEX_ATTRIBUTE_READ,
    AccessFlags::UNIFORM_READ,
    AccessFlags::SHADER_READ,
    AccessFlags::SHADER_WRITE,
    AccessFlags::TRANSFER_READ,
    AccessFlags::TRANSFER_WRITE,
];

/// Buffer resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Buffer;

impl Resource for Buffer {
    type Usage = BufferUsageFlags;
    type Layout = ();

    fn no_usage() -> Self::Usage {
        BufferUsageFlags::empty()
    }

    fn layout_for(_access: AccessFlags) {}

    fn valid_usage(access: AccessFlags, usage: BufferUsageFlags) -> bool {
        BUFFER_ACCESSES.iter().all(|&access_bit| {
            !access.subset(access_bit) || usage.intersects(match access_bit {
                AccessFlags::INDIRECT_COMMAND_READ => BufferUsageFlags::INDIRECT_BUFFER,
                AccessFlags::INDEX_READ => BufferUsageFlags::INDEX_BUFFER,
                AccessFlags::VERTEX_ATTRIBUTE_READ => BufferUsageFlags::VERTEX_BUFFER,
                AccessFlags::UNIFORM_READ => BufferUsageFlags::UNIFORM_BUFFER,
                AccessFlags::SHADER_READ => {
                    BufferUsageFlags::STORAGE_BUFFER
                        | BufferUsageFlags::UNIFORM_TEXEL_BUFFER
                        | BufferUsageFlags::STORAGE_TEXEL_BUFFER
                }
                AccessFlags::SHADER_WRITE => {
                    BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::STORAGE_TEXEL_BUFFER
                }
                AccessFlags::TRANSFER_READ => BufferUsageFlags::TRANSFER_SRC,
                AccessFlags::TRANSFER_WRITE => BufferUsageFlags::TRANSFER_DST,
                _ => unreachable!(),
            })
        })
    }
}

const IMAGE_ACCESSES: [AccessFlags; 9] = [
    AccessFlags::INPUT_ATTACHMENT_READ,
    AccessFlags::COLOR_ATTACHMENT_READ,
    AccessFlags::COLOR_ATTACHMENT_WRITE,
    AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
    AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
    AccessFlags::SHADER_READ,
    AccessFlags::SHADER_WRITE,
    AccessFlags::TRANSFER_READ,
    AccessFlags::TRANSFER_WRITE,
];

/// Image resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Image;

impl Resource for Image {
    type Usage = ImageUsageFlags;

    type Layout = ImageLayout;

    fn no_usage() -> Self::Usage {
        ImageUsageFlags::empty()
    }

    fn layout_for(access: AccessFlags) -> ImageLayout {
        IMAGE_ACCESSES
            .iter()
            .fold(None, |acc, &access_bit| {
                if access.subset(access_bit) {
                    let layout = match access_bit {
                        AccessFlags::INPUT_ATTACHMENT_READ => ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        AccessFlags::COLOR_ATTACHMENT_READ => ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        AccessFlags::COLOR_ATTACHMENT_WRITE => {
                            ImageLayout::COLOR_ATTACHMENT_OPTIMAL
                        }
                        AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ => {
                            ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL
                        }
                        AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE => {
                            ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
                        }
                        AccessFlags::TRANSFER_READ => ImageLayout::TRANSFER_SRC_OPTIMAL,
                        AccessFlags::TRANSFER_WRITE => ImageLayout::TRANSFER_DST_OPTIMAL,
                        _ => unreachable!(),
                    };
                    Some(match (acc, layout) {
                        (None, layout) => layout,
                        (Some(left), right) if left == right => left,
                        (
                            Some(ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL),
                            ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        ) => ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        (
                            Some(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
                            ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                        ) => ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        (Some(_), _) => ImageLayout::GENERAL,
                    })
                } else {
                    acc
                }
            }).unwrap_or(ImageLayout::GENERAL)
    }

    fn valid_usage(access: AccessFlags, usage: ImageUsageFlags) -> bool {
        IMAGE_ACCESSES.iter().all(|&access_bit| {
            !access.subset(access_bit) || usage.intersects(match access_bit {
                AccessFlags::INPUT_ATTACHMENT_READ => ImageUsageFlags::INPUT_ATTACHMENT,
                AccessFlags::COLOR_ATTACHMENT_READ => ImageUsageFlags::COLOR_ATTACHMENT,
                AccessFlags::COLOR_ATTACHMENT_WRITE => ImageUsageFlags::COLOR_ATTACHMENT,
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ => {
                    ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                }
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE => {
                    ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                }
                AccessFlags::TRANSFER_READ => ImageUsageFlags::TRANSFER_SRC,
                AccessFlags::TRANSFER_WRITE => ImageUsageFlags::TRANSFER_DST,
                _ => unreachable!(),
            })
        })
    }
}
