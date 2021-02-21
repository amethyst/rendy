use std::marker::PhantomData;

use rendy_core::{hal, Device, hal::device::Device as DeviceTrait};
use rendy_memory::{Heaps, MemoryBlock, MemoryUsage, Block, MappedRange};
use rendy_resource::{BufferInfo, CreationError};

use crate::{
    //ManagedDomain,
    handle::{HasValue, Handle},
    resource::Managed,
};

pub type ManagedBuffer<B> = Managed<BufferMarker<B>>;
pub struct BufferMarker<B>(PhantomData<B>) where B: hal::Backend;
impl<B> HasValue for BufferMarker<B> where B: hal::Backend {
    type Value = ManagedBufferData<B>;
}
pub type BufferHandle<B> = Handle<BufferMarker<B>>;

pub struct ManagedBufferData<B>
where
    B: hal::Backend,
{
    raw: B::Buffer,
    block: MemoryBlock<B>,
    info: BufferInfo,
}

impl<B: hal::Backend> ManagedBufferData<B> {

    pub fn create(
        device: &Device<B>,
        heaps: &mut Heaps<B>,
        //domain: ManagedDomain,
        info: BufferInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Self, CreationError<hal::buffer::CreationError>>
    {
        assert_ne!(info.size, 0);

        let mut buf = unsafe {
            device
                .create_buffer(info.size, info.usage)
                .map_err(CreationError::Create)?
        };
        let reqs = unsafe { device.get_buffer_requirements(&buf) };
        let block = heaps
            .allocate(
                device,
                reqs.type_mask as u32,
                memory_usage,
                reqs.size,
                reqs.alignment,
            )
            .map_err(CreationError::Allocate)?;

        unsafe {
            device
                .bind_buffer_memory(block.memory(), block.range().start, &mut buf)
                .map_err(CreationError::Bind)?;
        }

        let data = Self {
            raw: buf,
            block,
            info,
        };
        Ok(data)
    }

}

impl<B: hal::Backend> ManagedBuffer<B> {

    pub fn raw(&self) -> &B::Buffer {
        &self.inner.value.raw
    }
    /// Map range of the buffer to the CPU accessible memory.
    pub fn map<'a>(
        &'a mut self,
        device: &Device<B>,
        range: std::ops::Range<u64>,
    ) -> Result<MappedRange<'a, B>, rendy_core::hal::device::MapError> {
        self.inner.value.block.map(device, range)
    }

}
