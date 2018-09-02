
mod format;
mod usage;

pub use self::usage::UsageFlags;
pub use self::format::Format;

use std::cmp::max;

use memory::MemoryBlock;
use relevant::Relevant;

use device::Device;
use escape::Escape;
use SharingMode;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    D1,
    D2,
    D3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extent3D {
    width: u32,
    height: u32,
    depth: u32,
}

bitflags! {
    /// Bitmask specifying sample counts supported for an image used for storage operations.
    /// See Vulkan docs for detailed info:
    /// https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkSampleCountFlagBits.html
    #[repr(transparent)]
    pub struct SampleCountFlags: u32 {
        /// Specifies an image with one sample per pixel.
        const SAMPLE_COUNT_1 = 0x00000001;
        /// Specifies an image with 2 sample per pixel.
        const SAMPLE_COUNT_2 = 0x00000002;
        /// Specifies an image with 4 sample per pixel.
        const SAMPLE_COUNT_4 = 0x00000004;
        /// Specifies an image with 8 sample per pixel.
        const SAMPLE_COUNT_8 = 0x00000008;
        /// Specifies an image with 16 sample per pixel.
        const SAMPLE_COUNT_16 = 0x00000010;
        /// Specifies an image with 32 sample per pixel.
        const SAMPLE_COUNT_32 = 0x00000020;
        /// Specifies an image with 64 sample per pixel.
        const SAMPLE_COUNT_64 = 0x00000040;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageTiling {
    Optimal = 0,
    Linear = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Undefined = 0,
    General = 1,
    ColorAttachmentOptimal = 2,
    DepthStencilAttachmentOptimal = 3,
    DepthStencilReadOnlyOptimal = 4,
    ShaderReadOnlyOptimal = 5,
    TransferSrcOptimal = 6,
    TransferDstOptimal = 7,
    Preinitialized = 8,
    DepthReadOnlyStencilAttachmentOptimal = 1000117000,
    DepthAttachmentStencilReadOnlyOptimal = 1000117001,
    PresentSrc = 1000001002,
    SharedPresentSrc = 1000111000,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateInfo {
    kind: Kind,
    format: Format,
    extent: Extent3D,
    mips: u32,
    array: u32,
    samples: SampleCountFlags,
    tiling: ImageTiling,
    usage: UsageFlags,
    sharing: SharingMode,
}

#[derive(Debug)]
pub struct Image<T, I> {
    pub(super) inner: Escape<Inner<T, I>>,
    pub(super) info: CreateInfo,
}

#[derive(Debug)]
pub struct Inner<T, I> {
    pub(super) block: MemoryBlock<T>,
    pub(super) raw: I,
    pub(super) relevant: Relevant,
}
