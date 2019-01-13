use std::cmp::max;

use crate::{
    buffer,
    escape::Terminal,
    image,
    sampler::{Sampler, SamplerCache},
    memory::{Block, Heaps},
};

/// Resource manager.
/// It can be used to create and destroy resources such as buffers and images.
#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct Resources<B: gfx_hal::Backend> {
    buffers: Terminal<buffer::Inner<B>>,
    images: Terminal<image::Inner<B>>,
    image_views: Terminal<image::InnerView<B>>,
    sampler_cache: SamplerCache<B>,

    dropped_buffers: Vec<buffer::Inner<B>>,
    dropped_images: Vec<image::Inner<B>>,
    dropped_image_views: Vec<image::InnerView<B>>,
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
        &self,
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

        let mut buf = unsafe {
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

        unsafe {
            device.bind_buffer_memory(block.memory(), block.range().start, &mut buf)
        }?;

        Ok(unsafe { buffer::Buffer::new(buffer::Info {
                size,
                usage: usage.flags(),
            },
            buf,
            block,
            &self.buffers,
        )})
    }

    /// Destroy buffer.
    /// Buffer can be dropped but this method reduces overhead.
    pub fn destroy_buffer(&mut self, buffer: buffer::Buffer<B>) {
        buffer.unescape()
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
        let (raw, block) = inner.dispose();
        device.destroy_buffer(raw);
        heaps.free(device, block);
    }

    /// Create an image and bind to the memory that support intended usage.
    pub fn create_image(
        &self,
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
        assert!(
            levels <= kind.num_levels(),
            "Number of mip leves ({}) cannot be greater than {} for given kind {:?}",
            levels, kind.num_levels(), kind
        );

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

        let mut img = unsafe {
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

        unsafe {
            device.bind_image_memory(block.memory(), block.range().start, &mut img)
        }?;

        Ok(unsafe { image::Image::new(
            image::Info {
                kind,
                levels,
                format,
                tiling,
                view_caps,
                usage: usage.flags(),
            },
            img,
            Some(block),
            &self.images,
        )})
    }

    /// Create an image view.
    pub fn create_image_view(
        &self,
        device: &impl gfx_hal::Device<B>,
        image: &image::Image<B>,
        view_kind: gfx_hal::image::ViewKind,
        format: gfx_hal::format::Format,
        swizzle: gfx_hal::format::Swizzle,
        range: gfx_hal::image::SubresourceRange
    ) -> Result<image::ImageView<B>, failure::Error> {
        #[derive(Debug)] struct CreateImageView<'a> {
            image: &'a dyn std::fmt::Debug,
            view_kind: &'a dyn std::fmt::Debug,
            format: &'a dyn std::fmt::Debug,
            swizzle: &'a dyn std::fmt::Debug,
            range: &'a dyn std::fmt::Debug,
        };
        log::trace!("{:#?}", CreateImageView {
            image: &image,
            view_kind: &view_kind,
            format: &format,
            swizzle: &swizzle,
            range: &range,
        });

        let image_view = unsafe {
            device.create_image_view(
                image.raw(),
                view_kind,
                format,
                swizzle,
                gfx_hal::image::SubresourceRange {
                    aspects: range.aspects.clone(),
                    layers: range.layers.clone(),
                    levels: range.levels.clone(),
                },
            )
        }?;
        
        Ok(unsafe { image::ImageView::new(
            image::ViewInfo {
                view_kind,
                format,
                swizzle,
                range,
            },
            image,
            image_view,
            &self.image_views,
        )})
    }

    /// Create a sampler.
    pub fn create_sampler(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        filter: gfx_hal::image::Filter,
        wrap_mode: gfx_hal::image::WrapMode,
    ) -> Result<Sampler<B>, failure::Error> {
        Ok(self.sampler_cache.get(device, filter, wrap_mode))
    }

    /// Destroy image.
    /// Image can be dropped but this method reduces overhead.
    pub fn destroy_image(
        &mut self,
        image: image::Image<B>,
    ) {
        image.unescape()
            .map(|inner| self.dropped_images.push(inner));
    }

    /// Destroy image_view.
    /// Image_view can be dropped but this method reduces overhead.
    pub fn destroy_image_view(
        &mut self,
        image_view: image::ImageView<B>,
    ) {
        image_view.unescape()
            .map(|inner| self.dropped_image_views.push(inner));
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
        let (raw, block) = inner.dispose();
        device.destroy_image(raw);
        block.map(|block| heaps.free(device, block));
    }

    /// Drop inner image view representation.
    ///
    /// # Safety
    ///
    /// Device must not attempt to use the image view.
    unsafe fn actually_destroy_image_view(
        inner: image::InnerView<B>,
        device: &impl gfx_hal::Device<B>,
    ) {
        let (raw, image_kp) = inner.dispose();
        device.destroy_image_view(raw);
        drop(image_kp);
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

        for image_view in self.dropped_image_views.drain(..) {
            Self::actually_destroy_image_view(image_view, device);
        }

        for image in self.dropped_images.drain(..) {
            Self::actually_destroy_image(image, device, heaps);
        }

        self.sampler_cache.destroy(device);

        self.dropped_buffers.extend(self.buffers.drain());
        self.dropped_image_views.extend(self.image_views.drain());
        self.dropped_images.extend(self.images.drain());
    }
}
