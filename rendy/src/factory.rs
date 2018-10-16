use rendy_command::Families;
use rendy_memory::{Config, Heaps, MemoryError, Properties, Usage};
use rendy_resource::{
    buffer::{self, Buffer},
    image::{self, Image},
    ResourceError, Resources, SharingMode,
};

use device::Device;

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
    pub fn new<P, H>(device: D, types: P, heaps: H) -> Self
    where
        P: IntoIterator<Item = (Properties, u32, Config)>,
        H: IntoIterator<Item = u64>,
    {
        // TODO: make sure this is safe
        let heaps = unsafe { Heaps::new(types, heaps) };

        let families = Families::new();

        Factory {
            device,
            families,
            heaps,
            resources: Resources::new(),
        }
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
