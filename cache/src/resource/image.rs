use std::marker::PhantomData;

use rendy_core::{hal, Device, hal::device::Device as DeviceTrait};
use rendy_memory::{Heaps, MemoryBlock, MemoryUsage, Block};
use rendy_resource::{ImageInfo, CreationError};

use crate::{
    //ManagedDomain,
    handle::{HasValue, Handle},
    resource::Managed,
};

pub type ManagedImage<B> = Managed<ImageMarker<B>>;
pub struct ImageMarker<B>(PhantomData<B>) where B: hal::Backend;
impl<B> HasValue for ImageMarker<B> where B: hal::Backend {
    type Value = ManagedImageData<B>;
}
pub type ImageHandle<B> = Handle<ImageMarker<B>>;

pub struct ManagedImageData<B>
where
    B: hal::Backend,
{
    raw: B::Image,
    block: Option<MemoryBlock<B>>,
    info: ImageInfo,
}

impl<B: hal::Backend> ManagedImageData<B> {

    pub fn create(
        device: &Device<B>,
        heaps: &mut Heaps<B>,
        //domain: ManagedDomain,
        info: ImageInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Self, CreationError<hal::image::CreationError>>
    {
        // TODO: assert that info.levels <= info.kind.num_levels()

        let mut image = unsafe {
            device
                .create_image(
                    info.kind,
                    info.levels,
                    info.format,
                    info.tiling,
                    info.usage,
                    info.view_caps,
                )
                .map_err(CreationError::Create)?
        };
        let reqs = unsafe { device.get_image_requirements(&image) };
        let block = heaps
            .allocate(
                device,
                reqs.type_mask as u32,
                memory_usage,
                reqs.size,
                reqs.alignment,
            )
            .map_err(CreationError::Allocate)?;

        unsafe {
            device
                .bind_image_memory(block.memory(), block.range().start, &mut image)
                .map_err(CreationError::Bind)?;
        }

        let data = Self {
            raw: image,
            block: Some(block),
            info,
        };
        Ok(data)
    }

}

impl<B: hal::Backend> ManagedImage<B> {

    pub fn raw(&self) -> &B::Image {
        &self.inner.value.raw
    }

}
