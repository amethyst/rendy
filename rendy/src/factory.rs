use rendy_command::Families;
use rendy_memory::{Config as MemoryConfig, Heaps, MemoryError, Properties, Usage};
use rendy_resource::{
    buffer::{self, Buffer},
    image::{self, Image},
    ResourceError, Resources, SharingMode,
};

use config::Config;
use device::Device;
use physical_device::PhysicalDevice;
use queue::QueuesPicker;

pub struct Factory<D: Device> {
    device: D,
    families: Families<D::CommandQueue>,
    heaps: Heaps<D::Memory>,
    resources: Resources<D::Memory, D::Buffer, D::Image>,
}

impl<D> Factory<D>
where
    D: Device,
{
    pub fn new<P, H, Q>(config: Config<Q>, physical_device: P) -> Result<Self, ()>
    where
        P: PhysicalDevice<D>,
        Q: QueuesPicker,
    {
        let (device, families) = {
            let (family_id, count) = config.pick_queues()?;

            physical_device.open(family_id, count)?
        };

        let heaps = unimplemented!();

        Ok(Factory {
            device,
            families,
            heaps,
            resources: Resources::new(),
        })
    }

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
}
