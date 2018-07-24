
use std::ops::Range;
use hal; 

/// Memory block trait implemented for blocks allocated by allocators.
pub trait Block<T> {
    /// Get memory properties of the block.
    fn properties(&self) -> hal::memory::Properties;

    /// Get memory object.
    fn memory(&mut self) -> &mut T;

    /// Lock memory object.
    unsafe fn lock(&mut self);

    /// Unlock memory object.
    unsafe fn unlock(&mut self);

    /// Get memory range associated with this block.
    fn range(&self) -> Range<u64>;
}
