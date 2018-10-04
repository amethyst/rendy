//! Buffer usage, creation-info and wrappers.

mod usage;

pub use self::usage::*;
use memory::MemoryBlock;
use relevant::Relevant;

use escape::Escape;
use SharingMode;

/// Contains information required to create a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateInfo {
    /// Size of the buffer required.
    pub size: u64,

    /// Intended usage flags. Limits memory types suitable for the buffer.
    pub usage: UsageFlags,

    /// Specifies command queues from which families can access the buffer.
    pub sharing: SharingMode,
}

/// Generic buffer object wrapper.
///
/// # Parameters
///
/// `T` - type of the memory object of memory block.
/// `B` - raw buffer type.
#[derive(Debug)]
pub struct Buffer<M, B> {
    pub(crate) inner: Escape<Inner<M, B>>,
    pub(crate) info: CreateInfo,
}

#[derive(Debug)]
pub(crate) struct Inner<M, B> {
    pub(crate) block: MemoryBlock<M>,
    pub(crate) raw: B,
    pub(crate) relevant: Relevant,
}
