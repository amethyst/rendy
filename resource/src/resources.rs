use std::cmp::max;

use memory::{Block, Heaps, MemoryError, Usage as MemoryUsage};
use relevant::Relevant;

use buffer;
use device::Device;
use error::ResourceError;
use escape::{Escape, Terminal};
use image;

/// Resource manager.
/// It can be used to create and destroy resources such as buffers and images.
#[derive(Debug)]
pub struct Resources<T, B, I> {
    buffers: Terminal<buffer::Inner<T, B>>,
    images: Terminal<image::Inner<T, I>>,
}

impl<T: 'static, B: 'static, I: 'static> Resources<T, B, I> {
    /// Create a buffer and bind to the memory that support intended usage.
    pub fn create_buffer<D, M>(
        &mut self,
        device: &D,
        heaps: &mut Heaps<T>,
        info: buffer::CreateInfo,
        align: u64,
        memory_usage: M,
    ) -> Result<buffer::Buffer<T, B>, MemoryError>
    where
        D: Device<Memory = T, Buffer = B>,
        M: MemoryUsage,
    {
        let ubuf = device.create_buffer(info)?;
        let reqs = device.buffer_requirements(&ubuf);
        let block = heaps.allocate(
            device,
            reqs.mask,
            memory_usage,
            reqs.size,
            max(reqs.align, align),
        )?;

        let buf = unsafe {
            device
                .bind_buffer(ubuf, block.memory(), block.range().start)
                .unwrap()
        };

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
    pub unsafe fn destroy_buffer<D>(buffer: buffer::Buffer<T, B>, device: &D, heaps: &mut Heaps<T>)
    where
        D: Device<Memory = T, Buffer = B>,
    {
        Self::destroy_buffer_inner(Escape::into_inner(buffer.inner), device, heaps)
    }

    unsafe fn destroy_buffer_inner<D>(inner: buffer::Inner<T, B>, device: &D, heaps: &mut Heaps<T>)
    where
        D: Device<Memory = T, Buffer = B>,
    {
        device.destroy_buffer(inner.raw);
        heaps.free(device, inner.block);
    }

    /// Create an image and bind to the memory that support intended usage.
    pub fn create_image<D, M>(
        &mut self,
        device: &D,
        heaps: &mut Heaps<T>,
        info: image::CreateInfo,
        align: u64,
        memory_usage: M,
    ) -> Result<image::Image<T, I>, ResourceError>
    where
        D: Device<Memory = T, Image = I>,
        M: MemoryUsage,
    {
        let uimg = device.create_image(info)?;
        let reqs = device.image_requirements(&uimg);
        let block = heaps.allocate(
            device,
            reqs.mask,
            memory_usage,
            reqs.size,
            max(reqs.align, align),
        )?;

        let img = unsafe { device.bind_image(uimg, block.memory(), block.range().start)? };

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
    /// Buffer can be dropped but this method reduces overhead.
    pub unsafe fn destroy_image<D>(image: image::Image<T, I>, device: &D, heaps: &mut Heaps<T>)
    where
        D: Device<Memory = T, Image = I>,
    {
        Self::destroy_image_inner(Escape::into_inner(image.inner), device, heaps)
    }

    unsafe fn destroy_image_inner<D>(inner: image::Inner<T, I>, device: &D, heaps: &mut Heaps<T>)
    where
        D: Device<Memory = T, Image = I>,
    {
        device.destroy_image(inner.raw);
        heaps.free(device, inner.block);
    }

    /// Recycle dropped resources.
    pub unsafe fn cleanup<D>(&mut self, device: &D, heaps: &mut Heaps<T>)
    where
        D: Device<Memory = T, Buffer = B, Image = I>,
    {
        for buffer in self.buffers.drain() {
            device.destroy_buffer(buffer.raw);
            heaps.free(device, buffer.block);
        }

        for image in self.images.drain() {
            device.destroy_image(image.raw);
            heaps.free(device, image.block);
        }
    }
}
