
use block::Block;
use memory::Memory;

/// Allocator trait implemented for various allocators.
pub trait SubAllocator<T> {
    type Block: Block<T>;

    fn sub_allocate<F, E>(&mut self, size: u64, align: u64, external: F) -> Result<Self::Block, E>
    where
        F: FnMut(u64) -> Result<Memory<T>, E>,
    ;

    fn free<F>(&mut self, block: Self::Block, external: F)
    where
        F: FnMut(Memory<T>),
    ;
}
