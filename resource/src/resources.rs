use std::cmp::max;

use ash::{
    version::DeviceV1_0,
    vk,
};
use memory::{Block, Heaps, MemoryError, MemoryUsage};
use relevant::Relevant;

use buffer;
use escape::{Escape, Terminal};
use image;

/// Resource manager.
/// It can be used to create and destroy resources such as buffers and images.
#[derive(Debug, Default)]
pub struct Resources {
    buffers: Terminal<buffer::Inner>,
    images: Terminal<image::Inner>,
}

impl Resources {
    /// Create new `Resources` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a buffer and bind to the memory that support intended usage.
    pub fn create_buffer(
        &mut self,
        device: &impl DeviceV1_0,
        heaps: &mut Heaps,
        info: vk::BufferCreateInfo,
        align: u64,
        memory_usage: impl MemoryUsage,
    ) -> Result<buffer::Buffer, MemoryError> {
        let buf = unsafe { device.create_buffer(&info, None)? };
        let reqs = unsafe { device.get_buffer_memory_requirements(buf) };
        let block = heaps.allocate(
            device,
            reqs.memory_type_bits,
            memory_usage,
            reqs.size,
            max(reqs.alignment, align),
        )?;

        unsafe {
            device
                .bind_buffer_memory(buf, block.memory(), block.range().start)
                .unwrap()
        }

        Ok(buffer::Buffer {
            inner: self.buffers.escape(buffer::Inner {
                raw: buf,
                block,
                relevant: Relevant,
            }),
            info,
        })
    }

    /// Destroy buffer.
    /// Buffer can be dropped but this method reduces overhead.
    pub fn destroy_buffer(
        _buffer: buffer::Buffer,
        _device: &impl DeviceV1_0,
        _heaps: &mut Heaps,
    ) {
        unimplemented!()
    }

    /// Drop inner buffer representation.
    /// 
    /// # Safety
    /// 
    /// Device must not attempt to use the buffer.
    unsafe fn destroy_buffer_inner(
        inner: buffer::Inner,
        device: &impl DeviceV1_0,
        heaps: &mut Heaps,
    ) {
        device.destroy_buffer(inner.raw, None);
        heaps.free(device, inner.block);
        inner.relevant.dispose();
    }

    /// Create an image and bind to the memory that support intended usage.
    pub fn create_image(
        &mut self,
        device: &impl DeviceV1_0,
        heaps: &mut Heaps,
        info: vk::ImageCreateInfo,
        align: u64,
        memory_usage: impl MemoryUsage,
    ) -> Result<image::Image, MemoryError> {
        let img = unsafe { device.create_image(&info, None)? };
        let reqs = unsafe { device.get_image_memory_requirements(img) };
        let block = heaps.allocate(
            device,
            reqs.memory_type_bits,
            memory_usage,
            reqs.size,
            max(reqs.alignment, align),
        )?;

        unsafe {
            device
                .bind_image_memory(img, block.memory(), block.range().start)
                .unwrap()
        }

        Ok(image::Image {
            inner: self.images.escape(image::Inner {
                raw: img,
                block,
                relevant: Relevant,
            }),
            info,
        })
    }

    /// Destroy image.
    /// Image can be dropped but this method reduces overhead.
    pub unsafe fn destroy_image(
        _image: image::Image,
        _device: &impl DeviceV1_0,
        _heaps: &mut Heaps)
    {
        unimplemented!()
    }

    /// Drop inner image representation.
    /// 
    /// # Safety
    /// 
    /// Device must not attempt to use the image.
    unsafe fn destroy_image_inner(
        inner: image::Inner,
        device: &impl DeviceV1_0,
        heaps: &mut Heaps,
    ) {
        device.destroy_image(inner.raw, None);
        heaps.free(device, inner.block);
        inner.relevant.dispose();
    }

    /// Recycle dropped resources.
    /// 
    /// # Safety
    /// 
    /// Device must not attempt to use previously dropped buffers and images.
    pub unsafe fn cleanup(&mut self, device: &impl DeviceV1_0, heaps: &mut Heaps) {
        // trace!("Cleanup buffers");
        for buffer in self.buffers.drain() {
            Self::destroy_buffer_inner(buffer, device, heaps);
        }

        // trace!("Cleanup images");
        for image in self.images.drain() {
            Self::destroy_image_inner(image, device, heaps);
        }
    }
}
