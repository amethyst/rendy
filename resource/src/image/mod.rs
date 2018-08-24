
pub mod format;
pub mod usage;
use self::format::Format;
use self::usage::Flags;

use std::cmp::max;

use memory::SmartBlock;
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
    pub struct SampleCountFlags: u32 {
        const SAMPLE_COUNT_1 = 0x00000001;
        const SAMPLE_COUNT_2 = 0x00000002;
        const SAMPLE_COUNT_4 = 0x00000004;
        const SAMPLE_COUNT_8 = 0x00000008;
        const SAMPLE_COUNT_16 = 0x00000010;
        const SAMPLE_COUNT_32 = 0x00000020;
        const SAMPLE_COUNT_64 = 0x00000040;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageTiling {
    Optimal = 0,
    Linear = 1,
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
    usage: Flags,
    sharing: SharingMode,
}

#[derive(Debug)]
pub struct Image<T, I> {
    pub(super) inner: Escape<Inner<T, I>>,
    pub(super) info: CreateInfo,
}

#[derive(Debug)]
pub struct Inner<T, I> {
    pub(super) block: SmartBlock<T>,
    pub(super) raw: I,
    pub(super) relevant: Relevant,
}
