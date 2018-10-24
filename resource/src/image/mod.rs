//! Image usage, format, kind, extent, creation-info and wrappers.

mod usage;

pub use self::usage::*;

use ash::vk::{Image as AshImage, ImageCreateInfo};

use memory::MemoryBlock;
use relevant::Relevant;

use escape::Escape;

/// Generic image object wrapper.
///
/// # Parameters
///
/// `T` - type of the memory object of memory block.
/// `B` - raw image type.
#[derive(Debug)]
pub struct Image {
    pub(super) inner: Escape<Inner>,
    pub(super) info: ImageCreateInfo,
}

#[derive(Debug)]
pub(super) struct Inner {
    pub(super) block: MemoryBlock,
    pub(super) raw: AshImage,
    pub(super) relevant: Relevant,
}
