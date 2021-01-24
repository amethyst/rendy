//! Window system integration.

#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]
#![allow(clippy::missing_safety_doc)]

use rendy_core::{
    hal::{
        self,
        format::Format,
        window::{Extent2D, PresentationSurface, Surface as _, SurfaceCapabilities},
        Backend, Instance as _,
    },
    Device, HasRawWindowHandle, Instance, InstanceId,
};
use rendy_resource::{Image, ImageInfo};

/// Error creating a new swapchain.
#[derive(Debug)]
pub enum SwapchainError {
    /// Internal error in gfx-hal.
    Create(hal::window::CreationError),
    /// Present mode is not supported.
    BadPresentMode(hal::window::PresentMode),
    /// Image count is not supported.
    BadImageCount(hal::window::SwapImageIndex),
}

impl std::fmt::Display for SwapchainError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapchainError::Create(err) => {
                write!(
                    fmt,
                    "Failed to create swapchain because of a window creation error: {:?}",
                    err
                )
            }
            SwapchainError::BadPresentMode(present_mode) => {
                write!(
                fmt,
                "Failed to create swapchain because requested present mode is not supported: {:?}",
                present_mode
            )
            }
            SwapchainError::BadImageCount(image_count) => {
                write!(
                fmt,
                "Failed to create swapchain because requested image count is not supported: {:?}",
                image_count
            )
            }
        }
    }
}

impl std::error::Error for SwapchainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SwapchainError::Create(err) => Some(err),
            SwapchainError::BadPresentMode(_) => None,
            SwapchainError::BadImageCount(_) => None,
        }
    }
}

use derivative::Derivative;
/// Rendering target bound to window.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Surface<B: Backend> {
    #[derivative(Debug = "ignore")]
    raw: B::Surface,
    instance: InstanceId,
}

impl<B> Surface<B>
where
    B: Backend,
{
    /// Create surface for the window.
    pub fn new(
        instance: &Instance<B>,
        handle: &impl HasRawWindowHandle,
    ) -> Result<Self, hal::window::InitError> {
        let raw = unsafe { instance.create_surface(handle) }?;
        Ok(Surface {
            raw,
            instance: instance.id(),
        })
    }

    /// Create surface from `instance`.
    ///
    /// # Safety
    ///
    /// Closure must return surface object created from raw instance provided as closure argument.
    pub unsafe fn new_with(
        instance: &Instance<B>,
        f: impl FnOnce(&B::Instance) -> B::Surface,
    ) -> Self {
        Surface {
            raw: f(instance),
            instance: instance.id(),
        }
    }

    /// Create surface from raw parts.
    pub unsafe fn from_raw(surface: B::Surface, instance: InstanceId) -> Self {
        Surface {
            raw: surface,
            instance,
        }
    }
}

impl<B> Surface<B>
where
    B: Backend,
{
    /// Get raw `B::Surface` reference
    pub fn raw(&self) -> &B::Surface {
        &self.raw
    }

    /// Get raw mutable `B::Surface` reference
    pub fn raw_mut(&mut self) -> &mut B::Surface {
        &mut self.raw
    }

    /// Get current extent of the surface.
    pub unsafe fn extent(&self, physical_device: &B::PhysicalDevice) -> Option<Extent2D> {
        self.capabilities(physical_device).current_extent
    }

    /// Get surface ideal format.
    pub unsafe fn format(&self, physical_device: &B::PhysicalDevice) -> Format {
        if let Some(formats) = self.raw.supported_formats(physical_device) {
            *formats
                .iter()
                .max_by_key(|format| {
                    let base = format.base_format();
                    let desc = base.0.desc();
                    (
                        !desc.is_compressed(),
                        base.1 == hal::format::ChannelType::Srgb,
                        desc.bits,
                    )
                })
                .expect("At least one format must be supported by the surface")
        } else {
            Format::Rgba8Srgb
        }
    }

    /// Get formats supported by surface
    ///
    /// ## Safety
    ///
    /// - `physical_device` must be created from same `Instance` as the `Surface`
    pub unsafe fn supported_formats(
        &self,
        physical_device: &B::PhysicalDevice,
    ) -> Option<Vec<Format>> {
        self.raw.supported_formats(physical_device)
    }

    /// Get formats supported by surface
    ///
    /// ## Safety
    ///
    /// - `physical_device` must be created from same `Instance` as the `Surface`
    pub unsafe fn capabilities(&self, physical_device: &B::PhysicalDevice) -> SurfaceCapabilities {
        self.raw.capabilities(physical_device)
    }

    /// Cast surface into render target.
    pub unsafe fn into_target(
        mut self,
        physical_device: &B::PhysicalDevice,
        device: &Device<B>,
        suggest_extent: Extent2D,
        image_count: u32,
        present_mode: hal::window::PresentMode,
        usage: hal::image::Usage,
    ) -> Result<Target<B>, SwapchainError> {
        let extent = create_swapchain(
            &mut self,
            physical_device,
            device,
            suggest_extent,
            image_count,
            present_mode,
            usage,
        )?;

        Ok(Target {
            relevant: relevant::Relevant,
            surface: self,
            image_count,
            extent,
            present_mode,
            usage,
        })
    }
}

unsafe fn create_swapchain<B: Backend>(
    surface: &mut Surface<B>,
    physical_device: &B::PhysicalDevice,
    device: &Device<B>,
    suggest_extent: Extent2D,
    image_count: u32,
    present_mode: hal::window::PresentMode,
    usage: hal::image::Usage,
) -> Result<Extent2D, SwapchainError> {
    let capabilities = surface.capabilities(physical_device);
    let format = surface.format(physical_device);

    if !capabilities.present_modes.contains(present_mode) {
        return Err(SwapchainError::BadPresentMode(present_mode));
    }

    if image_count < *capabilities.image_count.start()
        || image_count > *capabilities.image_count.end()
    {
        return Err(SwapchainError::BadImageCount(image_count));
    }

    let extent = capabilities.current_extent.unwrap_or(suggest_extent);

    surface
        .raw_mut()
        .configure_swapchain(
            device,
            hal::window::SwapchainConfig {
                present_mode,
                format,
                extent,
                image_count,
                image_layers: 1,
                image_usage: usage,
                composite_alpha_mode: [
                    hal::window::CompositeAlphaMode::INHERIT,
                    hal::window::CompositeAlphaMode::OPAQUE,
                    hal::window::CompositeAlphaMode::PREMULTIPLIED,
                    hal::window::CompositeAlphaMode::POSTMULTIPLIED,
                ]
                .iter()
                .cloned()
                .find(|&bit| capabilities.composite_alpha_modes.contains(bit))
                .expect("No CompositeAlphaMode modes supported"),
            },
        )
        .map_err(SwapchainError::Create)?;

    Ok(extent)
}

/// Rendering target bound to window.
/// With swapchain created.
pub struct Target<B: Backend> {
    surface: Surface<B>,
    extent: Extent2D,
    image_count: u32,
    present_mode: hal::window::PresentMode,
    usage: hal::image::Usage,
    relevant: relevant::Relevant,
}

impl<B> std::fmt::Debug for Target<B>
where
    B: Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Target").finish()
    }
}

impl<B> Target<B>
where
    B: Backend,
{
    /// Dispose of target.
    ///
    /// # Safety
    ///
    /// Swapchain must be not in use.
    pub unsafe fn dispose(mut self, device: &Device<B>) -> Surface<B> {
        // if let Some(images) = self.backbuffer {
        //     images
        //         .into_iter()
        //         .for_each(|image| image.dispose_swapchain_image(device.id()));
        // }

        self.relevant.dispose();
        // if let Some(s) = self.swapchain.take() {
        //     device.destroy_swapchain(s)
        // }
        self.surface
    }

    /// Get raw surface handle.
    pub fn surface(&self) -> &Surface<B> {
        &self.surface
    }

    /// Recreate swapchain.
    ///
    /// #Safety
    ///
    /// Current swapchain must be not in use.
    pub unsafe fn recreate(
        &mut self,
        physical_device: &B::PhysicalDevice,
        device: &Device<B>,
        suggest_extent: Extent2D,
    ) -> Result<(), SwapchainError> {
        // let image_count = match self.backbuffer.take() {
        //     Some(images) => {
        //         let count = images.len();
        //         images
        //             .into_iter()
        //             .for_each(|image| image.dispose_swapchain_image(device.id()));
        //         count
        //     }
        //     None => 0,
        // };

        // if let Some(s) = self.swapchain.take() {
        //     device.destroy_swapchain(s)
        // }

        self.extent = create_swapchain(
            &mut self.surface,
            physical_device,
            device,
            suggest_extent,
            self.image_count,
            self.present_mode,
            self.usage,
        )?;

        Ok(())
    }

    /// Get render target size.
    pub fn extent(&self) -> Extent2D {
        self.extent
    }

    /// Get image usage flags.
    pub fn usage(&self) -> hal::image::Usage {
        self.usage
    }
}
