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

use {
    rendy_core::hal::{
        device::Device as _,
        format::Format,
        window::{Extent2D, Surface as _, SurfaceCapabilities},
        Backend, Instance as _,
    },
    rendy_core::{
        device_owned, instance_owned, Device, DeviceId, HasRawWindowHandle, Instance, InstanceId,
    },
    rendy_resource::{Image, ImageInfo},
};

/// Error creating a new swapchain.
#[derive(Debug)]
pub enum SwapchainError {
    /// Internal error in gfx-hal.
    Create(rendy_core::hal::window::CreationError),
    /// Present mode is not supported.
    BadPresentMode(rendy_core::hal::window::PresentMode),
    /// Image count is not supported.
    BadImageCount(rendy_core::hal::window::SwapImageIndex),
}

impl std::fmt::Display for SwapchainError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapchainError::Create(err) => write!(
                fmt,
                "Failed to create swapchain because of a window creation error: {:?}",
                err
            ),
            SwapchainError::BadPresentMode(present_mode) => write!(
                fmt,
                "Failed to create swapchain because requested present mode is not supported: {:?}",
                present_mode
            ),
            SwapchainError::BadImageCount(image_count) => write!(
                fmt,
                "Failed to create swapchain because requested image count is not supported: {:?}",
                image_count
            ),
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
/// Rendering target bound to window.
pub struct Surface<B: Backend> {
    raw: B::Surface,
    instance: InstanceId,
}

impl<B> std::fmt::Debug for Surface<B>
where
    B: Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Surface")
            .field("instance", &self.instance)
            .finish()
    }
}

instance_owned!(Surface<B>);

impl<B> Surface<B>
where
    B: Backend,
{
    /// Create surface for the window.
    pub fn new(
        instance: &Instance<B>,
        handle: &impl HasRawWindowHandle,
    ) -> Result<Self, rendy_core::hal::window::InitError> {
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
            raw: f(instance.raw()),
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
                        base.1 == rendy_core::hal::format::ChannelType::Srgb,
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

    // /// Cast surface into render target.
    // pub unsafe fn into_target(
    //     mut self,
    //     physical_device: &B::PhysicalDevice,
    //     device: &Device<B>,
    //     suggest_extent: Extent2D,
    //     image_count: u32,
    //     present_mode: rendy_core::hal::window::PresentMode,
    //     usage: rendy_core::hal::image::Usage,
    // ) -> Result<Target<B>, SwapchainError> {
    //     assert_eq!(
    //         device.id().instance,
    //         self.instance,
    //         "Resource is not owned by specified instance"
    //     );

    //     let (swapchain, backbuffer, extent) = create_swapchain(
    //         &mut self,
    //         physical_device,
    //         device,
    //         suggest_extent,
    //         image_count,
    //         present_mode,
    //         usage,
    //     )?;

    //     Ok(Target {
    //         device: device.id(),
    //         relevant: relevant::Relevant,
    //         surface: self,
    //         swapchain: Some(swapchain),
    //         backbuffer: Some(backbuffer),
    //         extent,
    //         present_mode,
    //         usage,
    //     })
    // }
}

// unsafe fn create_swapchain<B: Backend>(
//     surface: &mut Surface<B>,
//     physical_device: &B::PhysicalDevice,
//     device: &Device<B>,
//     suggest_extent: Extent2D,
//     image_count: u32,
//     present_mode: rendy_core::hal::window::PresentMode,
//     usage: rendy_core::hal::image::Usage,
// ) -> Result<(B::Swapchain, Vec<Image<B>>, Extent2D), SwapchainError> {
//     let capabilities = surface.capabilities(physical_device);
//     let format = surface.format(physical_device);
// 
//     if !capabilities.present_modes.contains(present_mode) {
//         log::warn!(
//             "Present mode is not supported. Supported: {:#?}, requested: {:#?}",
//             capabilities.present_modes,
//             present_mode,
//         );
//         return Err(SwapchainError::BadPresentMode(present_mode));
//     }
// 
//     log::trace!(
//         "Surface present modes: {:#?}. Pick {:#?}",
//         capabilities.present_modes,
//         present_mode
//     );
// 
//     log::trace!("Surface chosen format {:#?}", format);
// 
//     if image_count < *capabilities.image_count.start()
//         || image_count > *capabilities.image_count.end()
//     {
//         log::warn!(
//             "Image count not supported. Supported: {:#?}, requested: {:#?}",
//             capabilities.image_count,
//             image_count
//         );
//         return Err(SwapchainError::BadImageCount(image_count));
//     }
// 
//     log::trace!(
//         "Surface capabilities: {:#?}. Pick {} images",
//         capabilities.image_count,
//         image_count
//     );
// 
//     assert!(
//         capabilities.usage.contains(usage),
//         "Surface supports {:?}, but {:?} was requested",
//         capabilities.usage,
//         usage
//     );
// 
//     let extent = capabilities.current_extent.unwrap_or(suggest_extent);
// 
//     let (swapchain, images) = device
//         .create_swapchain(
//             &mut surface.raw,
//             rendy_core::hal::window::SwapchainConfig {
//                 present_mode,
//                 format,
//                 extent,
//                 image_count,
//                 image_layers: 1,
//                 image_usage: usage,
//                 composite_alpha_mode: [
//                     rendy_core::hal::window::CompositeAlphaMode::INHERIT,
//                     rendy_core::hal::window::CompositeAlphaMode::OPAQUE,
//                     rendy_core::hal::window::CompositeAlphaMode::PREMULTIPLIED,
//                     rendy_core::hal::window::CompositeAlphaMode::POSTMULTIPLIED,
//                 ]
//                 .iter()
//                 .cloned()
//                 .find(|&bit| capabilities.composite_alpha_modes.contains(bit))
//                 .expect("No CompositeAlphaMode modes supported"),
//             },
//             None,
//         )
//         .map_err(SwapchainError::Create)?;
// 
//     let backbuffer = images
//         .into_iter()
//         .map(|image| {
//             Image::create_from_swapchain(
//                 device.id(),
//                 ImageInfo {
//                     kind: rendy_core::hal::image::Kind::D2(extent.width, extent.height, 1, 1),
//                     levels: 1,
//                     format,
//                     tiling: rendy_core::hal::image::Tiling::Optimal,
//                     view_caps: rendy_core::hal::image::ViewCapabilities::empty(),
//                     usage,
//                 },
//                 image,
//             )
//         })
//         .collect();
// 
//     Ok((swapchain, backbuffer, extent))
// }
