
#![deny(unused_must_use)]

#[macro_use]
extern crate bitflags;

extern crate crossbeam_channel;
extern crate rendy_memory as memory;
extern crate relevant;

pub mod buffer;
pub mod image;
pub mod device;

mod escape;

use std::{cmp::max, marker::PhantomData};

use relevant::Relevant;
use memory::{Block, MemoryError, Heaps, SmartBlock};

use device::Device;
use escape::Escape;

/// Sharing mode.
/// Resources created with sharing mode `Exclusive`
/// can be accessed only from queues of single family that owns resource.
/// Resources created with sharing mode `Concurrent` can be accessed by queues
/// from specified families.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharingMode {
    Exclusive,
}

/// Memory requirements for the resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRequirements {
    /// Size of memory range required by the resource.
    pub size: u64,
    /// Minimal alignment required by the resource.
    pub align: u64,
    /// Memory type mask with bits set for memory types that support the resource.
    pub mask: u32,
}

#[derive(Debug)]
pub struct Resources<T, B, I> {
    buffers: escape::Terminal<buffer::Inner<T, B>>,
    images: escape::Terminal<image::Inner<T, I>>,
}


impl<T: 'static, B: 'static, I> Resources<T, B, I> {
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
        M: memory::Usage,
    {
        let ubuf = device.create_buffer(info)?;
        let reqs = device.buffer_requirements(&ubuf);
        let block = heaps.allocate(device, reqs.mask, memory_usage, reqs.size, max(reqs.align, align))?;

        let buf = unsafe {
            device.bind_buffer(ubuf, block.memory(), block.range().start)?
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
}


impl<T: 'static, B, I: 'static> Resources<T, B, I> {
    pub fn create<D, M>(
        &mut self,
        device: &D,
        heaps: &mut Heaps<T>,
        info: image::CreateInfo,
        align: u64,
        memory_usage: M,
    ) -> Result<image::Image<T, I>, MemoryError>
    where
        D: Device<Memory = T, Image = I>,
        M: memory::Usage,
    {
        let uimg = device.create_image(info)?;
        let reqs = device.image_requirements(&uimg);
        let block = heaps.allocate(device, reqs.mask, memory_usage, reqs.size, max(reqs.align, align))?;

        let img = unsafe {
            device.bind_image(uimg, block.memory(), block.range().start)?
        };

        Ok(image::Image {
            inner: self.images.escape(image::Inner {
                raw: img,
                block,
                relevant: Relevant,
            }),
            info,
        })
    }

    pub unsafe fn destroy_image<D>(image: image::Image<T, I>, device: &D, heaps: &mut Heaps<T>)
    where
        D: Device<Memory = T, Image = I>,
    {
        Self::destroy_image_inner(Escape::into_inner(image.inner), device, heaps)
    }

    unsafe fn destroy_image_inner<D>(inner: image::Inner<T, I>, device: &D, heaps: &mut Heaps<T>,)
    where
        D: Device<Memory = T, Image = I>,
    {
        device.destroy_image(inner.raw);
        heaps.free(device, inner.block);
    }
}


