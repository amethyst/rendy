use {
    crate::{
        buffer,
        escape::{KeepAlive, Terminal},
        image,
        memory::{Block, Heaps, MemoryBlock},
        sampler::{Sampler, SamplerCache},
    },
    smallvec::SmallVec,
    std::{cmp::max, collections::VecDeque},
};

/// Resource usage epochs.
#[derive(Clone, Debug)]
pub struct Epochs {
    pub values: SmallVec<[SmallVec<[u64; 8]>; 4]>,
}

impl Epochs {
    fn is_before(a: &Self, b: &Self) -> bool {
        debug_assert_eq!(a.values.len(), b.values.len());
        a.values.iter().zip(b.values.iter()).all(|(a, b)| {
            debug_assert_eq!(a.len(), b.len());
            a.iter().zip(b.iter()).all(|(a, b)| a < b)
        })
    }
}

/// Resource manager.
/// It can be used to create and destroy resources such as buffers and images.
#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct Resources<B: gfx_hal::Backend> {
    buffers: Terminal<(B::Buffer, MemoryBlock<B>)>,
    images: Terminal<(B::Image, Option<MemoryBlock<B>>)>,
    image_views: Terminal<(B::ImageView, KeepAlive)>,
    sampler_cache: SamplerCache<B>,

    dropped_buffers: VecDeque<(Epochs, B::Buffer, MemoryBlock<B>)>,
    dropped_images: VecDeque<(Epochs, B::Image, Option<MemoryBlock<B>>)>,
    dropped_image_views: VecDeque<(Epochs, B::ImageView, KeepAlive)>,
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
        #[derive(Debug)]
        struct CreateBuffer<'a> {
            align: &'a dyn std::fmt::Debug,
            size: &'a dyn std::fmt::Debug,
            usage: &'a dyn std::fmt::Debug,
        };
        log::trace!(
            "{:#?}",
            CreateBuffer {
                align: &align,
                size: &size,
                usage: &usage,
            }
        );

        let mut buf = unsafe { device.create_buffer(size, usage.flags()) }?;
        let reqs = unsafe { device.get_buffer_requirements(&buf) };
        let block = heaps.allocate(
            device,
            reqs.type_mask as u32,
            usage.memory(),
            reqs.size,
            max(reqs.alignment, align),
        )?;

        unsafe { device.bind_buffer_memory(block.memory(), block.range().start, &mut buf) }?;

        Ok(unsafe {
            buffer::Buffer::new(
                buffer::Info {
                    size,
                    usage: usage.flags(),
                },
                buf,
                block,
                &self.buffers,
            )
        })
    }

    // /// Destroy buffer.
    // /// Buffer can be dropped but this method reduces overhead.
    // pub fn destroy_buffer(&mut self, buffer: buffer::Buffer<B>) {
    //     buffer
    //         .unescape()
    //         .map(|(raw, block)| self.dropped_buffers.push(inner));
    // }

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
            levels,
            kind.num_levels(),
            kind
        );

        #[derive(Debug)]
        struct CreateImage<'a> {
            align: &'a dyn std::fmt::Debug,
            kind: &'a dyn std::fmt::Debug,
            levels: &'a dyn std::fmt::Debug,
            format: &'a dyn std::fmt::Debug,
            tiling: &'a dyn std::fmt::Debug,
            view_caps: &'a dyn std::fmt::Debug,
            usage: &'a dyn std::fmt::Debug,
        };
        log::trace!(
            "{:#?}",
            CreateImage {
                align: &align,
                kind: &kind,
                levels: &levels,
                format: &format,
                tiling: &tiling,
                view_caps: &view_caps,
                usage: &usage,
            }
        );

        let mut img =
            unsafe { device.create_image(kind, levels, format, tiling, usage.flags(), view_caps) }?;
        let reqs = unsafe { device.get_image_requirements(&img) };
        let block = heaps.allocate(
            device,
            reqs.type_mask as u32,
            usage.memory(),
            reqs.size,
            max(reqs.alignment, align),
        )?;

        unsafe { device.bind_image_memory(block.memory(), block.range().start, &mut img) }?;

        Ok(unsafe {
            image::Image::new(
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
            )
        })
    }

    /// Create an image view.
    pub fn create_image_view(
        &self,
        device: &impl gfx_hal::Device<B>,
        image: &image::Image<B>,
        view_kind: gfx_hal::image::ViewKind,
        format: gfx_hal::format::Format,
        swizzle: gfx_hal::format::Swizzle,
        range: gfx_hal::image::SubresourceRange,
    ) -> Result<image::ImageView<B>, failure::Error> {
        #[derive(Debug)]
        struct CreateImageView<'a> {
            image: &'a dyn std::fmt::Debug,
            view_kind: &'a dyn std::fmt::Debug,
            format: &'a dyn std::fmt::Debug,
            swizzle: &'a dyn std::fmt::Debug,
            range: &'a dyn std::fmt::Debug,
        };
        log::trace!(
            "{:#?}",
            CreateImageView {
                image: &image,
                view_kind: &view_kind,
                format: &format,
                swizzle: &swizzle,
                range: &range,
            }
        );

        let image_info = image.info();
        assert!(match_kind(image_info.kind, view_kind, image_info.view_caps));

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

        Ok(unsafe {
            image::ImageView::new(
                image::ViewInfo {
                    view_kind,
                    format,
                    swizzle,
                    range,
                },
                image,
                image_view,
                &self.image_views,
            )
        })
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

    // /// Destroy image.
    // /// Image can be dropped but this method reduces overhead.
    // pub fn destroy_image(
    //     &mut self,
    //     image: image::Image<B>,
    // ) {
    //     image.unescape()
    //         .map(|inner| self.dropped_images.push(inner));
    // }

    // /// Destroy image_view.
    // /// Image_view can be dropped but this method reduces overhead.
    // pub fn destroy_image_view(
    //     &mut self,
    //     image_view: image::ImageView<B>,
    // ) {
    //     image_view.unescape()
    //         .map(|inner| self.dropped_image_views.push(inner));
    // }

    /// Recycle dropped resources.
    pub unsafe fn cleanup(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        heaps: &mut Heaps<B>,
        complete: Epochs,
        next: Epochs,
    ) {
        log::trace!("Cleanup resources");

        while let Some((epoch, raw, block)) = self.dropped_buffers.pop_front() {
            if Epochs::is_before(&epoch, &complete) {
                self.dropped_buffers.push_front((epoch, raw, block));
                break;
            }

            device.destroy_buffer(raw);
            heaps.free(device, block);
        }

        while let Some((epoch, raw, kp)) = self.dropped_image_views.pop_front() {
            if Epochs::is_before(&epoch, &complete) {
                self.dropped_image_views.push_front((epoch, raw, kp));
                break;
            }

            device.destroy_image_view(raw);
            drop(kp);
        }

        while let Some((epoch, raw, block)) = self.dropped_images.pop_front() {
            if Epochs::is_before(&epoch, &complete) {
                self.dropped_images.push_front((epoch, raw, block));
                break;
            }

            device.destroy_image(raw);
            block.map(|block| heaps.free(device, block));
        }

        self.dropped_buffers.extend(
            self.buffers
                .drain()
                .map(|(raw, block)| (next.clone(), raw, block)),
        );
        self.dropped_image_views.extend(
            self.image_views
                .drain()
                .map(|(raw, block)| (next.clone(), raw, block)),
        );
        self.dropped_images
            .extend(self.images.drain().map(|(raw, kp)| (next.clone(), raw, kp)));
    }

    /// Destroy all dropped resources.
    ///
    /// # Safety
    ///
    /// Device must be idle.
    pub unsafe fn dispose(mut self, device: &impl gfx_hal::Device<B>, heaps: &mut Heaps<B>) {
        log::trace!("Dispose of all resources");
        for (raw, block) in self
            .dropped_buffers
            .drain(..)
            .map(|(_, raw, block)| (raw, block))
            .chain(self.buffers.drain())
        {
            device.destroy_buffer(raw);
            heaps.free(device, block);
        }

        for (raw, kp) in self
            .dropped_image_views
            .drain(..)
            .map(|(_, raw, kp)| (raw, kp))
            .chain(self.image_views.drain())
        {
            device.destroy_image_view(raw);
            drop(kp);
        }

        for (raw, block) in self
            .dropped_images
            .drain(..)
            .map(|(_, raw, block)| (raw, block))
            .chain(self.images.drain())
        {
            device.destroy_image(raw);
            block.map(|block| heaps.free(device, block));
        }

        self.sampler_cache.destroy(device);
    }
}

fn match_kind(
    kind: gfx_hal::image::Kind,
    view_kind: gfx_hal::image::ViewKind,
    view_caps: gfx_hal::image::ViewCapabilities,
) -> bool {
    match kind {
        gfx_hal::image::Kind::D1(..) => match view_kind {
            gfx_hal::image::ViewKind::D1 | gfx_hal::image::ViewKind::D1Array => true,
            _ => false,
        },
        gfx_hal::image::Kind::D2(..) => match (view_kind, view_caps) {
            (gfx_hal::image::ViewKind::Cube, caps) if caps.contains(gfx_hal::image::ViewCapabilities::KIND_CUBE) => true,
            (gfx_hal::image::ViewKind::D2, _) | (gfx_hal::image::ViewKind::D2Array, _) => true,
            _ => false,
        },
        gfx_hal::image::Kind::D3(..) => {
            if view_caps == gfx_hal::image::ViewCapabilities::KIND_2D_ARRAY {
                if view_kind == gfx_hal::image::ViewKind::D2 {
                    true
                } else if view_kind == gfx_hal::image::ViewKind::D2Array {
                    true
                } else {
                    false
                }
            } else if view_kind == gfx_hal::image::ViewKind::D3 {
                true
            } else {
                false
            }
        }
    }
}
