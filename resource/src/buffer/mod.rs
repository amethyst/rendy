//! Buffer usage, creation-info and wrappers.

mod usage;

use ash::vk::{Buffer as AshBuffer, BufferCreateInfo};

pub use self::usage::*;
use memory::MemoryBlock;
use relevant::Relevant;

use escape::Escape;

/// Generic buffer object wrapper.
///
/// # Parameters
///
/// `T` - type of the memory object of memory block.
/// `B` - raw buffer type.
#[derive(Debug)]
pub struct Buffer {
    pub(crate) inner: Escape<Inner>,
    pub(crate) info: BufferCreateInfo,
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) block: MemoryBlock,
    pub(crate) raw: AshBuffer,
    pub(crate) relevant: Relevant,
}
