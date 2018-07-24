
use std::ops::Range;
use hal;
use block::Block;
use memory::Memory;
use sub::SubAllocator;

pub struct DedicatedAllocator;
pub struct DedicatedBlock<T>(Memory<T>);

impl<T> Block<T> for DedicatedBlock<T> {

    #[inline]
    fn properties(&self) -> hal::memory::Properties {
        self.0.properties()
    }

    #[inline]
    fn memory(&mut self) -> &mut T {
        self.0.raw_mut()
    }

    #[inline]
    unsafe fn lock(&mut self) { /*Not shared*/ }

    #[inline]
    unsafe fn unlock(&mut self) { /*Not shared*/ }

    #[inline]
    fn range(&self) -> Range<u64> {
        0 .. self.0.size()
    }
}

impl<T> SubAllocator<T> for DedicatedAllocator {
    type Block = DedicatedBlock<T>;

    #[inline]
    fn sub_allocate<F, E>(&mut self, size: u64, _align: u64, external: F) -> Result<DedicatedBlock<T>, E>
    where
        F: FnOnce(u64) -> Result<Memory<T>, E>,
    {
        external(size).map(DedicatedBlock)
    }

    #[inline]
    fn free<F>(&mut self, block: Self::Block, external: F)
    where
        F: FnOnce(Memory<T>),
    {
        external(block.0)
    }
}

