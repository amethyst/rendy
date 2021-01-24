//! Image usage, format, kind, extent, creation-info and wrappers.

use relevant::Relevant;
pub use rendy_core::hal::image::*;
use rendy_core::{
    hal::{self, device::Device as _, format, Backend},
    Device, DeviceId,
};

use crate::{
    escape::Handle,
    memory::{Block, Heaps, MemoryBlock, MemoryUsage},
    CreationError,
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

/// Alias for the error to create an image.
pub type ImageCreationError = CreationError<hal::image::CreationError>;

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

use derive_more::{Deref, DerefMut};

/// Generic image view resource wrapper.
#[derive(Debug, Deref, DerefMut)]
pub struct ImageView<B: Backend> {
    #[deref]
    #[deref_mut]
    raw: B::ImageView,
    image: Handle<Image<B>>,
    info: ImageViewInfo,
    relevant: Relevant,
}

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

    /// Get image view info.
    pub fn info(&self) -> &ImageViewInfo {
        &self.info
    }

    /// Get image of this view.
    pub fn image(&self) -> &Handle<Image<B>> {
        &self.image
    }
}
