//! Image usage, format, kind, extent, creation-info and wrappers.

pub use rendy_core::hal::image::*;

use {
    crate::{
        core::{device_owned, Device, DeviceId},
        escape::Handle,
        memory::{Block, Heaps, MemoryBlock, MemoryUsage},
        CreationError,
    },
    relevant::Relevant,
    rendy_core::hal::{device::Device as _, format, Backend},
};

/// Image info.
#[derive(Clone, Copy, Debug)]
pub struct ImageInfo {
    /// Kind of the image.
    pub kind: Kind,

    /// Image mip-level count.
    pub levels: Level,

    /// Image format.
    pub format: format::Format,

    /// Image tiling mode.
    pub tiling: Tiling,

    /// Image view capabilities.
    pub view_caps: ViewCapabilities,

    /// Image usage flags.
    pub usage: Usage,
}

/// Generic image resource wrapper.
///
/// # Parameters
///
/// `B` - raw image type.
#[derive(Debug)]
pub struct Image<B: Backend> {
    device: DeviceId,
    raw: B::Image,
    block: Option<MemoryBlock<B>>,
    info: ImageInfo,
    relevant: Relevant,
}

device_owned!(Image<B>);
/// Alias for the error to create an image.
pub type ImageCreationError = CreationError<rendy_core::hal::image::CreationError>;

impl<B> Image<B>
where
    B: Backend,
{
    /// Create image.
    ///
    /// # Safety
    ///
    /// In order to guarantee that `Heap::allocate` will return
    /// memory range owned by this `Device`,
    /// this `Heaps` instance must always be used with this `Device` instance.
    ///
    /// Otherwise usage of hal methods must be always valid.
    pub unsafe fn create(
        device: &Device<B>,
        heaps: &mut Heaps<B>,
        info: ImageInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Self, ImageCreationError> {
        assert!(
            info.levels <= info.kind.num_levels(),
            "Number of mip leves ({}) cannot be greater than {} for given kind {:?}",
            info.levels,
            info.kind.num_levels(),
            info.kind,
        );

        log::trace!("{:#?}@{:#?}", info, memory_usage);

        let mut img = device
            .create_image(
                info.kind,
                info.levels,
                info.format,
                info.tiling,
                info.usage,
                info.view_caps,
            )
            .map_err(CreationError::Create)?;
        let reqs = device.get_image_requirements(&img);
        let block = heaps
            .allocate(
                device,
                reqs.type_mask as u32,
                memory_usage,
                reqs.size,
                reqs.alignment,
            )
            .map_err(CreationError::Allocate)?;

        device
            .bind_image_memory(block.memory(), block.range().start, &mut img)
            .map_err(CreationError::Bind)?;

        Ok(Image {
            device: device.id(),
            raw: img,
            block: Some(block),
            info,
            relevant: Relevant,
        })
    }

    /// Create image handler for swapchain image.
    pub unsafe fn create_from_swapchain(device: DeviceId, info: ImageInfo, raw: B::Image) -> Self {
        Image {
            device,
            raw,
            block: None,
            info,
            relevant: Relevant,
        }
    }

    /// Destroy image resource.
    pub unsafe fn dispose(self, device: &Device<B>, heaps: &mut Heaps<B>) {
        device.destroy_image(self.raw);
        if let Some(block) = self.block {
            heaps.free(device, block)
        }
        self.relevant.dispose();
    }

    /// Drop image wrapper for swapchain image.
    pub unsafe fn dispose_swapchain_image(self, _device: DeviceId) {
        self.relevant.dispose();
    }

    /// Get reference for raw image resource.
    pub fn raw(&self) -> &B::Image {
        &self.raw
    }

    /// Get mutable reference for raw image resource.
    pub unsafe fn raw_mut(&mut self) -> &mut B::Image {
        &mut self.raw
    }

    /// Get reference to memory block occupied by image.
    pub fn block(&self) -> Option<&MemoryBlock<B>> {
        self.block.as_ref()
    }

    /// Get mutable reference to memory block occupied by image.
    pub unsafe fn block_mut(&mut self) -> Option<&mut MemoryBlock<B>> {
        self.block.as_mut()
    }

    /// Get image info.
    pub fn info(&self) -> &ImageInfo {
        &self.info
    }

    /// Get [`Kind`] of the image.
    ///
    /// [`Kind`]: ../gfx-hal/image/struct.Kind.html
    pub fn kind(&self) -> Kind {
        self.info.kind
    }

    /// Get [`Format`] of the image.
    ///
    /// [`Format`]: ../gfx-hal/format/struct.Format.html
    pub fn format(&self) -> format::Format {
        self.info.format
    }

    /// Get levels count of the image.
    pub fn levels(&self) -> u8 {
        self.info.levels
    }

    /// Get layers count of the image.
    pub fn layers(&self) -> u16 {
        self.info.kind.num_layers()
    }
}

/// Image view info
#[derive(Clone, Debug)]
pub struct ImageViewInfo {
    /// View kind
    pub view_kind: ViewKind,
    /// Format for this view
    pub format: format::Format,
    /// Swizzle operator for this view
    pub swizzle: format::Swizzle,
    /// Range of full image to view
    pub range: SubresourceRange,
}

/// Generic image view resource wrapper.
#[derive(Debug)]
pub struct ImageView<B: Backend> {
    raw: B::ImageView,
    image: Handle<Image<B>>,
    info: ImageViewInfo,
    relevant: Relevant,
}

device_owned!(ImageView<B> @ |view: &Self| view.image.device_id());
/// Alias for the error to create an image view.
pub type ImageViewCreationError = CreationError<ViewCreationError>;

impl<B> ImageView<B>
where
    B: Backend,
{
    /// Create an image view.
    pub fn create(
        device: &Device<B>,
        info: ImageViewInfo,
        image: Handle<Image<B>>,
    ) -> Result<Self, ImageViewCreationError> {
        log::trace!("{:#?}@{:#?}", info, image);

        assert!(match_kind(
            image.kind(),
            info.view_kind,
            image.info().view_caps
        ));

        let view = unsafe {
            device
                .create_image_view(
                    image.raw(),
                    info.view_kind,
                    info.format,
                    info.swizzle,
                    SubresourceRange {
                        aspects: info.range.aspects,
                        layers: info.range.layers.clone(),
                        levels: info.range.levels.clone(),
                    },
                )
                .map_err(CreationError::Create)?
        };

        Ok(ImageView {
            raw: view,
            image,
            info,
            relevant: Relevant,
        })
    }

    /// Destroy image view resource.
    pub unsafe fn dispose(self, device: &Device<B>) {
        device.destroy_image_view(self.raw);
        drop(self.image);
        self.relevant.dispose();
    }

    /// Get reference to raw image view resoruce.
    pub fn raw(&self) -> &B::ImageView {
        &self.raw
    }

    /// Get mutable reference to raw image view resoruce.
    pub unsafe fn raw_mut(&mut self) -> &mut B::ImageView {
        &mut self.raw
    }

    /// Get image view info.
    pub fn info(&self) -> &ImageViewInfo {
        &self.info
    }

    /// Get image of this view.
    pub fn image(&self) -> &Handle<Image<B>> {
        &self.image
    }
}

fn match_kind(kind: Kind, view_kind: ViewKind, view_caps: ViewCapabilities) -> bool {
    match kind {
        Kind::D1(..) => match view_kind {
            ViewKind::D1 | ViewKind::D1Array => true,
            _ => false,
        },
        Kind::D2(..) => match view_kind {
            ViewKind::D2 | ViewKind::D2Array => true,
            ViewKind::Cube => view_caps.contains(ViewCapabilities::KIND_CUBE),
            _ => false,
        },
        Kind::D3(..) => {
            if view_caps == ViewCapabilities::KIND_2D_ARRAY {
                view_kind == ViewKind::D2 || view_kind == ViewKind::D2Array
            } else {
                view_kind == ViewKind::D3
            }
        }
    }
}
