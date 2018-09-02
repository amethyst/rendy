

mod usage;

pub use self::usage::UsageFlags;
use memory::MemoryBlock;
use relevant::Relevant;

use device::Device;
use escape::Escape;
use SharingMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateInfo {
    pub size: u64,
    pub usage: UsageFlags,
    pub sharing: SharingMode,
}

#[derive(Debug)]
pub struct Buffer<T, B> {
    pub(super) inner: Escape<Inner<T, B>>,
    pub(super) info: CreateInfo,
}

#[derive(Debug)]
pub struct Inner<T, B> {
    pub(super) block: MemoryBlock<T>,
    pub(super) raw: B,
    pub(super) relevant: Relevant,
}
