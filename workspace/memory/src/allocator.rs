
use hal;
use block::Block;

pub trait Allocator<B: hal::Backend> {
    type Block: Block<B::Memory>;

    fn allocate_from(&mut self, device: &B::Device, memory_type_id: hal::adapter::MemoryTypeId, size: u64, align: u64) -> Result<Self::Block, hal::device::OutOfMemory>;
    fn allocate_with(&mut self, device: &B::Device, mask: u64, properties: hal::memory::Properties, size: u64, align: u64) -> Result<Self::Block, hal::device::OutOfMemory>;
    fn free(&mut self, device: &B::Device, block: Self::Block);
}

