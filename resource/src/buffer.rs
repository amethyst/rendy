//! Buffer usage, creation-info and wrappers.

pub use rendy_core::hal::buffer::*;

use {
    crate::{
        core::{device_owned, Device, DeviceId},
        memory::{Block, Heaps, MappedRange, MemoryBlock, MemoryUsage},
        CreationError,
    },
    relevant::Relevant,
    rendy_core::hal::{device::Device as _, Backend},
};

/// Buffer info.
#[derive(Clone, Copy, Debug)]
pub struct BufferInfo {
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
    device: DeviceId,
    raw: B::Buffer,
    block: MemoryBlock<B>,
    info: BufferInfo,
    relevant: Relevant,
}

device_owned!(Buffer<B>);
/// Alias for the error to create a buffer.
pub type BufferCreationError = CreationError<rendy_core::hal::buffer::CreationError>;

impl<B> Buffer<B>
where
    B: Backend,
{
    /// Create buffer, allocate memory block for it and bind.
    ///
    /// # Safety
    ///
    /// In order to guarantee that `Heap::allocate` will return
    /// memory range owned by this `Device`,
    /// this `Heaps` instance must always be used with this `Device` instance.
    ///
    /// Otherwise usage of hal methods must be always valid.
    pub unsafe fn create(
        device: &Device<B>,
        heaps: &mut Heaps<B>,
        info: BufferInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Self, BufferCreationError> {
        log::trace!("{:#?}@{:#?}", info, memory_usage);
        assert_ne!(info.size, 0);

        let mut buf = device
            .create_buffer(info.size, info.usage)
            .map_err(CreationError::Create)?;
        let reqs = device.get_buffer_requirements(&buf);
        let block = heaps
            .allocate(
                device,
                reqs.type_mask as u32,
                memory_usage,
                reqs.size,
                reqs.alignment,
            )
            .map_err(CreationError::Allocate)?;

        device
            .bind_buffer_memory(block.memory(), block.range().start, &mut buf)
            .map_err(CreationError::Bind)?;

        Ok(Buffer {
            device: device.id(),
            raw: buf,
            block,
            info,
            relevant: Relevant,
        })
    }

    /// Dispose of buffer resource.
    /// # Safety
    /// Deallocate memory block.
    pub unsafe fn dispose(self, device: &Device<B>, heaps: &mut Heaps<B>) {
        self.assert_device_owner(device);
        device.destroy_buffer(self.raw);
        heaps.free(device, self.block);
        self.relevant.dispose();
    }

    /// Get reference to raw buffer resource
    pub fn raw(&self) -> &B::Buffer {
        &self.raw
    }

    /// Get mutable reference to raw buffer resource
    pub unsafe fn raw_mut(&mut self) -> &mut B::Buffer {
        &mut self.raw
    }

    /// Get reference to memory block occupied by buffer.
    pub fn block(&self) -> &MemoryBlock<B> {
        &self.block
    }

    /// Get mutable reference to memory block occupied by buffer.
    pub unsafe fn block_mut(&mut self) -> &mut MemoryBlock<B> {
        &mut self.block
    }

    /// Get buffer info.
    pub fn info(&self) -> &BufferInfo {
        &self.info
    }

    /// Check if this buffer could is bound to CPU visible memory and therefore mappable.
    /// If this function returns `false` `map` will always return `InvalidAccess`.
    ///
    /// [`map`]: #method.map
    /// [`InvalidAccess`]: https://docs.rs/gfx-hal/0.1/rendy_core::hal/mapping/enum.Error.html#InvalidAccess
    pub fn visible(&self) -> bool {
        self.block
            .properties()
            .contains(rendy_core::hal::memory::Properties::CPU_VISIBLE)
    }

    /// Map range of the buffer to the CPU accessible memory.
    pub fn map<'a>(
        &'a mut self,
        device: &Device<B>,
        range: std::ops::Range<u64>,
    ) -> Result<MappedRange<'a, B>, rendy_core::hal::device::MapError> {
        self.block.map(device, range)
    }

    /// Get buffer info.
    pub fn size(&self) -> u64 {
        self.info().size
    }
}
