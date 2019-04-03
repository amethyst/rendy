//! Buffer usage, creation-info and wrappers.

pub use gfx_hal::buffer::*;

use {
    crate::memory::{Block, Heaps, MappedRange, MemoryBlock, MemoryUsage},
    gfx_hal::{Backend, Device as _},
    relevant::Relevant,
};

/// Buffer info.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    /// Buffer size.
    pub size: u64,

    /// Buffer usage flags.
    pub usage: Usage,
}

/// Generic buffer resource wrapper.
///
/// # Parameters
///
/// `B` - raw image type.
#[derive(Debug)]
pub struct Buffer<B: Backend> {
    raw: B::Buffer,
    block: MemoryBlock<B>,
    info: Info,
    relevant: Relevant,
}

impl<B> Buffer<B>
where
    B: Backend,
{
    /// Create buffer, allocate memory block for it and bind.
    pub unsafe fn create(
        device: &B::Device,
        heaps: &mut Heaps<B>,
        info: Info,
        memory_usage: impl MemoryUsage,
    ) -> Result<Self, failure::Error> {
        log::trace!("{:#?}@{:#?}", info, memory_usage);

        let mut buf = device.create_buffer(info.size, info.usage)?;
        let reqs = device.get_buffer_requirements(&buf);
        let block = heaps.allocate(
            device,
            reqs.type_mask as u32,
            memory_usage,
            reqs.size,
            reqs.alignment,
        )?;

        device.bind_buffer_memory(block.memory(), block.range().start, &mut buf)?;

        Ok(Buffer {
            raw: buf,
            block,
            info,
            relevant: Relevant,
        })
    }

    pub unsafe fn dispose(self, device: &B::Device, heaps: &mut Heaps<B>) {
        device.destroy_buffer(self.raw);
        heaps.free(device, self.block);
        self.relevant.dispose();
    }

    pub fn raw(&self) -> &B::Buffer {
        &self.raw
    }

    pub unsafe fn raw_mut(&mut self) -> &mut B::Buffer {
        &mut self.raw
    }

    pub fn block(&self) -> &MemoryBlock<B> {
        &self.block
    }

    pub unsafe fn block_mut(&mut self) -> &mut MemoryBlock<B> {
        &mut self.block
    }

    pub fn info(&self) -> &Info {
        &self.info
    }

    /// Check if this buffer could is bound to CPU visible memory and therefore mappable.
    /// If this function returns `false` `map` will always return `InvalidAccess`.
    ///
    /// [`map`]: #method.map
    /// [`InvalidAccess`]: https://docs.rs/gfx-hal/0.1/gfx_hal/mapping/enum.Error.html#InvalidAccess
    pub fn visible(&self) -> bool {
        self.block
            .properties()
            .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
    }

    /// Map range of the buffer to the CPU accessible memory.
    pub fn map<'a>(
        &'a mut self,
        device: &impl gfx_hal::Device<B>,
        range: std::ops::Range<u64>,
    ) -> Result<MappedRange<'a, B>, gfx_hal::mapping::Error> {
        self.block.map(device, range)
    }

    /// Get buffer info.
    pub fn size(&self) -> u64 {
        self.info().size
    }
}
