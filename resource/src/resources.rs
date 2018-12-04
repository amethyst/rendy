use std::cmp::max;

use crate::{
    buffer,
    escape::{Escape, Terminal},
    image,
    memory::{Block, Heaps},
};

/// Resource manager.
/// It can be used to create and destroy resources such as buffers and images.
#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct Resources<B: gfx_hal::Backend> {
    buffers: Terminal<buffer::Inner<B>>,
    images: Terminal<image::Inner<B>>,

    dropped_buffers: Vec<buffer::Inner<B>>,
    dropped_images: Vec<image::Inner<B>>,
}

impl<B> Resources<B>
where
    B: gfx_hal::Backend,
{
    /// Create new `Resources` instance.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a buffer and bind to the memory that support intended usage.
    pub fn create_buffer(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        heaps: &mut Heaps<B>,
        align: u64,
        size: u64,
        usage: impl buffer::Usage,
    ) -> Result<buffer::Buffer<B>, failure::Error> {
        #[derive(Debug)] struct CreateBuffer<'a> {
            align: &'a dyn std::fmt::Debug,
            size: &'a dyn std::fmt::Debug,
            usage: &'a dyn std::fmt::Debug,
        };
        log::trace!("{:#?}", CreateBuffer {
            align: &align,
            size: &size,
            usage: &usage,
        });

        let buf = unsafe {
            device.create_buffer(size, usage.flags())
        }?;
        let reqs = unsafe {
            device.get_buffer_requirements(&buf)
        };
        let block = heaps.allocate(
            device,
            reqs.type_mask as u32,
            usage.memory(),
            reqs.size,
            max(reqs.alignment, align),
        )?;

        let buf = unsafe {
            device.bind_buffer_memory(block.memory(), block.range().start, buf)
        }?;

        Ok(buffer::Buffer {
            escape: self.buffers.escape(buffer::Inner {
                raw: buf,
                block,
                relevant: relevant::Relevant,
            }),
            info: buffer::Info {
                size,
                usage: usage.flags(),
            }
        })
    }

    /// Destroy buffer.
    /// Buffer can be dropped but this method reduces overhead.
    pub fn destroy_buffer(&mut self, buffer: buffer::Buffer<B>) {
        Escape::dispose(buffer.escape)
            .map(|inner| self.dropped_buffers.push(inner));
    }

    /// Drop inner buffer representation.
    ///
    /// # Safety
    ///
    /// Device must not attempt to use the buffer.
    unsafe fn actually_destroy_buffer(
        inner: buffer::Inner<B>,
        device: &impl gfx_hal::Device<B>,
        heaps: &mut Heaps<B>,
    ) {
        device.destroy_buffer(inner.raw);
        heaps.free(device, inner.block);
        inner.relevant.dispose();
    }

    /// Create an image and bind to the memory that support intended usage.
    pub fn create_image(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        heaps: &mut Heaps<B>,
        align: u64,
        kind: gfx_hal::image::Kind,
        levels: gfx_hal::image::Level,
        format: gfx_hal::format::Format,
        tiling: gfx_hal::image::Tiling,
        view_caps: gfx_hal::image::ViewCapabilities,
        usage: impl image::Usage,
    ) -> Result<image::Image<B>, failure::Error> {
        #[derive(Debug)] struct CreateImage<'a> {
            align: &'a dyn std::fmt::Debug,
            kind: &'a dyn std::fmt::Debug,
            levels: &'a dyn std::fmt::Debug,
            format: &'a dyn std::fmt::Debug,
            tiling: &'a dyn std::fmt::Debug,
            view_caps: &'a dyn std::fmt::Debug,
            usage: &'a dyn std::fmt::Debug,
        };
        log::trace!("{:#?}", CreateImage {
            align: &align,
            kind: &kind,
            levels: &levels,
            format: &format,
            tiling: &tiling,
            view_caps: &view_caps,
            usage: &usage,
        });

        let img = unsafe {
            device.create_image(
                kind,
                levels,
                format,
                tiling,
                usage.flags(),
                view_caps,
            )
        }?;
        let reqs = unsafe {
            device.get_image_requirements(&img)
        };
        let block = heaps.allocate(
            device,
            reqs.type_mask as u32,
            usage.memory(),
            reqs.size,
            max(reqs.alignment, align),
        )?;

        let img = unsafe {
            device
                .bind_image_memory(block.memory(), block.range().start, img)
        }?;

        Ok(image::Image {
            escape: self.images.escape(image::Inner {
                raw: img,
                block,
                relevant: relevant::Relevant,
            }),
            info: image::Info {
                kind,
                levels,
                format,
                tiling,
                view_caps,
                usage: usage.flags(),
            },
        })
    }

    /// Destroy image.
    /// Image can be dropped but this method reduces overhead.
    pub fn destroy_image(
        &mut self,
        image: image::Image<B>,
    ) {
        Escape::dispose(image.escape)
            .map(|inner| self.dropped_images.push(inner));
    }

    /// Drop inner image representation.
    ///
    /// # Safety
    ///
    /// Device must not attempt to use the image.
    unsafe fn actually_destroy_image(
        inner: image::Inner<B>,
        device: &impl gfx_hal::Device<B>,
        heaps: &mut Heaps<B>,
    ) {
        device.destroy_image(inner.raw);
        heaps.free(device, inner.block);
        inner.relevant.dispose();
    }

    /// Recycle dropped resources.
    ///
    /// # Safety
    ///
    /// Device must not attempt to use previously dropped buffers and images.
    pub unsafe fn cleanup(&mut self, device: &impl gfx_hal::Device<B>, heaps: &mut Heaps<B>) {
        log::trace!("Cleanup resources");
        for buffer in self.dropped_buffers.drain(..) {
            Self::actually_destroy_buffer(buffer, device, heaps);
        }

        for image in self.dropped_images.drain(..) {
            Self::actually_destroy_image(image, device, heaps);
        }

        self.dropped_buffers.extend(self.buffers.drain());
        self.dropped_images.extend(self.images.drain());
    }
}
