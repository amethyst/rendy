
use std::{any::Any, cmp::max, fmt::Debug, sync::Arc};
use hal;
use memory::{Allocator, Block};
use relevant::Relevant;
use escape::Escape;
use Resources;

#[derive(Debug)]
pub struct Buffer<B: hal::Backend, T> {
    inner: Escape<Inner<B, T>>,
}

impl<B: hal::Backend, T> Buffer<B, T> {
    pub fn create<A>(
        device: &B::Device,
        resources: &Resources<B, T>,
        memory: &mut A,
        usage: hal::buffer::Usage,
        size: u64,
        align: u64,
        properties: hal::memory::Properties,
    ) -> Result<Self, hal::buffer::CreationError>
    where
        T: Block<B::Memory>,
        A: Allocator<B, Block = T>,
    {
        Ok(Buffer {
            inner: resources.buffer.escape(Inner::create(
                device,
                memory,
                usage,
                size,
                align,properties,
            )?)
        })
    }
}

#[derive(Debug)]
pub struct Inner<B: hal::Backend, T> {
    raw: B::Buffer,
    block: T,
    relevant: Relevant,
}

impl<B: hal::Backend, T> Inner<B, T> {
    pub fn create<A>(
        device: &B::Device,
        memory: &mut A,
        usage: hal::buffer::Usage,
        size: u64,
        align: u64,
        properties: hal::memory::Properties,
    ) -> Result<Self, hal::buffer::CreationError>
    where
        T: Block<B::Memory>,
        A: Allocator<B, Block = T>,
    {
        let buffer = hal::Device::create_buffer(device, size, usage)?;
        let requirements = hal::Device::get_buffer_requirements(device, &buffer);
        assert!(requirements.size >= size);
        let align = max(align, requirements.alignment);
        let mut block = memory.allocate_with(
            device,
            requirements.type_mask,
            properties,
            requirements.size,
            align,
        )
        .map_err(|_| hal::buffer::CreationError::OutOfDeviceMemory)?;

        let offset = block.range().start;
        let buffer = hal::Device::bind_buffer_memory(device, block.memory(), offset, buffer).unwrap();

        Ok(Inner {
            raw: buffer,
            block,
            relevant: Relevant,
        })
    }

    pub unsafe fn destroy<A>(self, device: &B::Device, memory: &mut A)
    where
        T: Block<B::Memory>,
        A: Allocator<B, Block = T>,
    {
        hal::Device::destroy_buffer(device, self.raw);
        memory.free(device, self.block);
    }
}

