use std::{
    fmt::Debug,
    ops::{BitOr, BitOrAssign},
};

use ash::vk;

/// Abstracts resource types that uses different usage flags and layouts types.
pub trait Resource: 'static {
    /// Usage flags type for the resource.
    type Usage: Copy + Debug + BitOr<Output = Self::Usage> + BitOrAssign + 'static;

    /// Layout type for the resource.
    type Layout: Copy + Debug + 'static;

    /// Empty usage.
    fn no_usage() -> Self::Usage;

    /// Layout suitable for specified accesses.
    fn layout_for(access: vk::AccessFlags) -> Self::Layout;

    /// Check if all usage flags required for access are set.
    fn valid_usage(access: vk::AccessFlags, usage: Self::Usage) -> bool;
}

const BUFFER_ACCESSES: [vk::AccessFlags; 8] = [
    vk::AccessFlags::INDIRECT_COMMAND_READ,
    vk::AccessFlags::INDEX_READ,
    vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
    vk::AccessFlags::UNIFORM_READ,
    vk::AccessFlags::SHADER_READ,
    vk::AccessFlags::SHADER_WRITE,
    vk::AccessFlags::TRANSFER_READ,
    vk::AccessFlags::TRANSFER_WRITE,
];

/// Buffer resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Buffer;

impl Resource for Buffer {
    type Usage = vk::BufferUsageFlags;
    type Layout = ();

    fn no_usage() -> Self::Usage {
        vk::BufferUsageFlags::empty()
    }

    fn layout_for(_access: vk::AccessFlags) {}

    fn valid_usage(access: vk::AccessFlags, usage: vk::BufferUsageFlags) -> bool {
        BUFFER_ACCESSES.iter().all(|&access_bit| {
            !access.subset(access_bit) || usage.intersects(match access_bit {
                vk::AccessFlags::INDIRECT_COMMAND_READ => vk::BufferUsageFlags::INDIRECT_BUFFER,
                vk::AccessFlags::INDEX_READ => vk::BufferUsageFlags::INDEX_BUFFER,
                vk::AccessFlags::VERTEX_ATTRIBUTE_READ => vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::AccessFlags::UNIFORM_READ => vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::AccessFlags::SHADER_READ => {
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER
                        | vk::BufferUsageFlags::STORAGE_TEXEL_BUFFER
                }
                vk::AccessFlags::SHADER_WRITE => {
                    vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::STORAGE_TEXEL_BUFFER
                }
                vk::AccessFlags::TRANSFER_READ => vk::BufferUsageFlags::TRANSFER_SRC,
                vk::AccessFlags::TRANSFER_WRITE => vk::BufferUsageFlags::TRANSFER_DST,
                _ => unreachable!(),
            })
        })
    }
}

const IMAGE_ACCESSES: [vk::AccessFlags; 9] = [
    vk::AccessFlags::INPUT_ATTACHMENT_READ,
    vk::AccessFlags::COLOR_ATTACHMENT_READ,
    vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
    vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
    vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
    vk::AccessFlags::SHADER_READ,
    vk::AccessFlags::SHADER_WRITE,
    vk::AccessFlags::TRANSFER_READ,
    vk::AccessFlags::TRANSFER_WRITE,
];

/// Image resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Image;

impl Resource for Image {
    type Usage = vk::ImageUsageFlags;

    type Layout = vk::ImageLayout;

    fn no_usage() -> Self::Usage {
        vk::ImageUsageFlags::empty()
    }

    fn layout_for(access: vk::AccessFlags) -> vk::ImageLayout {
        IMAGE_ACCESSES
            .iter()
            .fold(None, |acc, &access_bit| {
                if access.subset(access_bit) {
                    let layout = match access_bit {
                        vk::AccessFlags::INPUT_ATTACHMENT_READ => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        vk::AccessFlags::COLOR_ATTACHMENT_READ => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        vk::AccessFlags::COLOR_ATTACHMENT_WRITE => {
                            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
                        }
                        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ => {
                            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL
                        }
                        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE => {
                            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
                        }
                        vk::AccessFlags::TRANSFER_READ => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        vk::AccessFlags::TRANSFER_WRITE => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        _ => unreachable!(),
                    };
                    Some(match (acc, layout) {
                        (None, layout) => layout,
                        (Some(left), right) if left == right => left,
                        (
                            Some(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL),
                            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        ) => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        (
                            Some(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
                            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                        ) => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        (Some(_), _) => vk::ImageLayout::GENERAL,
                    })
                } else {
                    acc
                }
            }).unwrap_or(vk::ImageLayout::GENERAL)
    }

    fn valid_usage(access: vk::AccessFlags, usage: vk::ImageUsageFlags) -> bool {
        IMAGE_ACCESSES.iter().all(|&access_bit| {
            !access.subset(access_bit) || usage.intersects(match access_bit {
                vk::AccessFlags::INPUT_ATTACHMENT_READ => vk::ImageUsageFlags::INPUT_ATTACHMENT,
                vk::AccessFlags::COLOR_ATTACHMENT_READ => vk::ImageUsageFlags::COLOR_ATTACHMENT,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE => vk::ImageUsageFlags::COLOR_ATTACHMENT,
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ => {
                    vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                }
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE => {
                    vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                }
                vk::AccessFlags::TRANSFER_READ => vk::ImageUsageFlags::TRANSFER_SRC,
                vk::AccessFlags::TRANSFER_WRITE => vk::ImageUsageFlags::TRANSFER_DST,
                _ => unreachable!(),
            })
        })
    }
}
