use command::Families;
use memory::{Config as MemoryConfig, Heaps, MemoryError, Properties, Usage};
use resource::{
    buffer::{self, Buffer},
    image::{self, Image},
    ResourceError, Resources, SharingMode,
};
use winit::Window;

use config::{Config, RenderConfig};
use device::Device;
use queue::QueuesPicker;
use render::{Render, Target};

/// The `Factory<D>` type represents the overall creation type for `rendy`.
pub struct Factory<D: Device> {
    pub device: D,
    families: Families<D::CommandQueue>,
    heaps: Heaps<D::Memory>,
    resources: Resources<D::Memory, D::Buffer, D::Image>,
}

impl<D> Factory<D>
where
    D: Device,
{
    /// Creates a new `Factory` based off of a `Config<Q, W>` with some `QueuesPicker`
    /// from the specified `PhysicalDevice`.
    pub fn new<P, Q>(config: Config, queue_picker: Q) -> Result<Factory<D>, ()>
    where
        Q: QueuesPicker,
    {
        let heaps = unimplemented!();
        let device = unimplemented!();
        let families = unimplemented!();

        Ok(Factory {
            device,
            families,
            heaps,
            resources: Resources::new(),
        })
    }

    /// Creates a buffer that is managed with the specified properties.
    pub fn create_buffer<U>(
        &mut self,
        size: u64,
        usage: buffer::UsageFlags,
        sharing: SharingMode,
        align: u64,
        memory_usage: U,
    ) -> Result<Buffer<D::Memory, D::Buffer>, MemoryError>
    where
        U: Usage,
    {
        let info = buffer::CreateInfo {
            size,
            usage,
            sharing,
        };

        self.resources
            .create_buffer(&self.device, &mut self.heaps, info, align, memory_usage)
    }

    /// Creates an image that is mananged with the specified properties.
    pub fn create_image<U>(
        &mut self,
        kind: image::Kind,
        format: image::Format,
        extent: image::Extent3D,
        mips: u32,
        array: u32,
        samples: image::SampleCountFlags,
        tiling: image::ImageTiling,
        usage: image::UsageFlags,
        flags: image::ImageCreateFlags,
        sharing: SharingMode,
        align: u64,
        memory_usage: U,
    ) -> Result<Image<D::Memory, D::Image>, ResourceError>
    where
        U: Usage,
    {
        let info = image::CreateInfo {
            kind,
            format,
            extent,
            mips,
            array,
            samples,
            tiling,
            usage,
            flags,
            sharing,
        };

        self.resources
            .create_image(&self.device, &mut self.heaps, info, align, memory_usage)
    }

    // pub fn create_surface<R>(window: &Window) -> Target<D, R> {
    //     unimplemented!()
    // }

    // /// Build a `Render<D, T>` from the `RenderBuilder` and a render info
    // pub fn build_render<'a, R, T>(builder: RenderBuilder, render_config: RenderConfig) -> R
    // where
    //     R: Render<D, T>,
    // {
    //     unimplemented!()
    // }
}
