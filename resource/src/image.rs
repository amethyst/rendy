
use std::{any::Any, cmp::max, fmt::Debug, sync::Arc};
use hal;
use memory::{Allocator, Block};
use relevant::Relevant;
use escape::Escape;
use Resources;

#[derive(Debug)]
pub struct Image<B: hal::Backend, T> {
    inner: Escape<Inner<B, T>>,
}

impl<B: hal::Backend, T> Image<B, T> {
    pub fn create<A>(
        device: &B::Device,
        resources: &Resources<B, T>,
        memory: &mut A,
        usage: hal::image::Usage,
        kind: hal::image::Kind,
        levels: u8,
        format: hal::format::Format,
        tiling: hal::image::Tiling,
        flags: hal::image::StorageFlags,
        align: u64,
        properties: hal::memory::Properties,
    ) -> Result<Self, hal::image::CreationError>
    where
        T: Block<B::Memory>,
        A: Allocator<B, Block = T>,
    {
        Ok(Image {
            inner: resources.image.escape(Inner::create(
                device,
                memory,
                usage,
                kind,levels,
                format,tiling,
                flags,
                align,
                properties,
            )?),
        })
    }

    pub fn raw(&self) -> &B::Image {
        &self.inner.raw
    }
}

#[derive(Debug)]
pub struct Inner<B: hal::Backend, T> {
    raw: B::Image,
    block: T,
    relevant: Relevant,
}

impl<B: hal::Backend, T> Inner<B, T> {
    pub fn create<A>(
        device: &B::Device,
        memory: &mut A,
        usage: hal::image::Usage,
        kind: hal::image::Kind,
        levels: u8,
        format: hal::format::Format,
        tiling: hal::image::Tiling,
        flags: hal::image::StorageFlags,
        align: u64,
        properties: hal::memory::Properties,
    ) -> Result<Self, hal::image::CreationError>
    where
        T: Block<B::Memory>,
        A: Allocator<B, Block = T>,
    {
        let image = hal::Device::create_image(device, kind, levels, format, tiling, usage, flags)?;
        let requirements = hal::Device::get_image_requirements(device, &image);
        let align = max(align, requirements.alignment);
        let mut block = memory.allocate_with(
            device,
            requirements.type_mask,
            properties,
            requirements.size,
            align,
        )
        .map_err(|_| unimplemented!("Target error doesn't have OOM error variant"))?;

        let offset = block.range().start;
        let image = hal::Device::bind_image_memory(device, block.memory(), offset, image).unwrap();

        Ok(Inner {
            raw: image,
            block,
            relevant: Relevant,
        })
    }

    pub unsafe fn destroy<A>(self, device: &B::Device, memory: &mut A)
    where
        T: Block<B::Memory>,
        A: Allocator<B, Block = T>,
    {
        hal::Device::destroy_image(device, self.raw);
        memory.free(device, self.block);
    }

    pub fn raw(&self) -> &B::Image {
        &self.raw
    }
}

#[derive(Debug)]
pub struct ImageView<B: hal::Backend, T> {
    raw: B::ImageView,
    image: Arc<Image<B, T>>,
}

impl<B: hal::Backend, T> ImageView<B, T> {
    pub fn create(
        device: &B::Device,
        image: Arc<Image<B, T>>,
        view_kind: hal::image::ViewKind,
        format: hal::format::Format,
        swizzle: hal::format::Swizzle,
        range: hal::image::SubresourceRange,
    ) -> Result<Self, hal::image::ViewError>
    {
        Ok(ImageView {
            raw: hal::Device::create_image_view(device, image.raw(), view_kind, format, swizzle, range)?,
            image: image.clone(),
        })
    }
}

