
use std::{fmt::Debug, ops::{BitOr, BitOrAssign}};
use rendy_resource::{buffer, image};

use access::AccessFlags;

pub trait Resource: 'static {
    type Usage: Copy + Debug + BitOr<Output = Self::Usage> + BitOrAssign + 'static;
    type Layout: Copy + Debug + 'static;

    fn no_usage() -> Self::Usage;

    fn layout_for(access: AccessFlags) -> Self::Layout;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Buffer;
impl Resource for Buffer {
    type Usage = buffer::UsageFlags;
    type Layout = ();

    fn no_usage() -> Self::Usage {
        buffer::UsageFlags::empty()
    }

    fn layout_for(_access: AccessFlags) {}

    fn valid_usage(access: AccessFlags, usage: buffer::UsageFlags) -> bool {
        BUFFER_ACCESSES.iter().all(|&access_bit| {
            !access.contains(access_bit) || usage.intersects(match access_bit {
                AccessFlags::INDIRECT_COMMAND_READ => buffer::UsageFlags::INDIRECT_BUFFER,
                AccessFlags::INDEX_READ => buffer::UsageFlags::INDEX_BUFFER,
                AccessFlags::VERTEX_ATTRIBUTE_READ => buffer::UsageFlags::VERTEX_BUFFER,
                AccessFlags::UNIFORM_READ => buffer::UsageFlags::UNIFORM_BUFFER,
                AccessFlags::SHADER_READ => buffer::UsageFlags::STORAGE_BUFFER | buffer::UsageFlags::UNIFORM_TEXEL_BUFFER | buffer::UsageFlags::STORAGE_TEXEL_BUFFER,
                AccessFlags::SHADER_WRITE => buffer::UsageFlags::STORAGE_BUFFER | buffer::UsageFlags::STORAGE_TEXEL_BUFFER,
                AccessFlags::TRANSFER_READ => buffer::UsageFlags::TRANSFER_SRC,
                AccessFlags::TRANSFER_WRITE => buffer::UsageFlags::TRANSFER_DST,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Image;
impl Resource for Image {
    type Usage = image::UsageFlags;
    type Layout = image::Layout;

    fn no_usage() -> Self::Usage {
        image::UsageFlags::empty()
    }

    fn layout_for(access: AccessFlags) -> image::Layout {
        IMAGE_ACCESSES.iter().fold(None, |acc, &access_bit| {
            if access.contains(access_bit) {
                let layout = match access_bit {
                    AccessFlags::INPUT_ATTACHMENT_READ => image::Layout::ShaderReadOnlyOptimal,
                    AccessFlags::COLOR_ATTACHMENT_READ => image::Layout::ColorAttachmentOptimal,
                    AccessFlags::COLOR_ATTACHMENT_WRITE => image::Layout::ColorAttachmentOptimal,
                    AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ => image::Layout::DepthStencilReadOnlyOptimal,
                    AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE => image::Layout::DepthStencilAttachmentOptimal,
                    AccessFlags::TRANSFER_READ => image::Layout::TransferSrcOptimal,
                    AccessFlags::TRANSFER_WRITE => image::Layout::TransferDstOptimal,
                    _ => unreachable!(),
                };
                Some(match (acc, layout) {
                    (None, layout) => layout,
                    (Some(left), right) if left == right => left,
                    (Some(image::Layout::DepthStencilReadOnlyOptimal), image::Layout::DepthStencilAttachmentOptimal) => image::Layout::DepthStencilAttachmentOptimal,
                    (Some(image::Layout::DepthStencilAttachmentOptimal), image::Layout::DepthStencilReadOnlyOptimal) => image::Layout::DepthStencilAttachmentOptimal,
                    (Some(_), _) => image::Layout::General,
                })
            } else {
                acc
            }
        }).unwrap_or(image::Layout::General)
    }

    fn valid_usage(access: AccessFlags, usage: image::UsageFlags) -> bool {
        IMAGE_ACCESSES.iter().all(|&access_bit| {
            !access.contains(access_bit) || usage.intersects(match access_bit {
                AccessFlags::INPUT_ATTACHMENT_READ => image::UsageFlags::INPUT_ATTACHMENT,
                AccessFlags::COLOR_ATTACHMENT_READ => image::UsageFlags::COLOR_ATTACHMENT,
                AccessFlags::COLOR_ATTACHMENT_WRITE => image::UsageFlags::COLOR_ATTACHMENT,
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ => image::UsageFlags::DEPTH_STENCIL_ATTACHMENT,
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE => image::UsageFlags::DEPTH_STENCIL_ATTACHMENT,
                AccessFlags::TRANSFER_READ => image::UsageFlags::TRANSFER_SRC,
                AccessFlags::TRANSFER_WRITE => image::UsageFlags::TRANSFER_DST,
                _ => unreachable!(),
            })
        })
    }
}
