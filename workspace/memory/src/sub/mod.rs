
pub mod arena;
pub mod dedicated;

use std::{fmt::Debug, ops::Range};
use hal;
use block::Block;
use memory::Memory;

/// Allocator trait implemented for various allocators.
pub trait SubAllocator<T: Debug + Send + Sync + 'static> {
    type Block: Block<T>;

    fn sub_allocate<B, F, E>(&mut self, device: &B::Device, size: u64, align: u64, external: F) -> Result<Self::Block, E>
    where
        B: hal::Backend<Memory = T>,
        F: FnMut(u64) -> Result<Memory<T>, E>,
        E: From<hal::device::OutOfMemory>,
    ;

    fn free<B, F>(&mut self, device: &B::Device, block: Self::Block, external: F)
    where
        B: hal::Backend<Memory = T>,
        F: FnMut(Memory<T>),
    ;
}

#[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
fn fits_in_usize(value: u64) -> bool {
    value <= usize::max_value() as u64
}

#[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))]
fn fits_in_usize(_: u64) -> bool {
    true
}

fn aligned(value: u64, align: u64) -> u64 {
    1 + (value - 1) | (align - 1)
}

/// Check if `sub` fit within `range`.
fn sub_range(range: Range<u64>, sub: Range<u64>) -> bool {
    range.start <= sub.start && range.end >= sub.end
}

fn clamp_range(values: Range<u64>, range: Range<u64>) -> Range<u64> {
    use std::cmp::{min, max};
    min(range.end, max(range.start, values.start)) .. min(range.end, max(range.start, values.end))
}
