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

use hal::window::AcquireError;
use hal::window::PresentationSurface;
use relevant::Relevant;
use rendy_core::hal;

use {
    hal::{
        format::Format,
        window::{Extent2D, Surface as _, SurfaceCapabilities},
        Backend, Instance as _,
    },
    rendy_core::{
        device_owned, instance_owned, Device, DeviceId, HasRawWindowHandle, Instance, InstanceId,
    },
    rendy_resource::Image,
};

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
        assert_eq!(
            device.id().instance,
            self.instance,
            "Resource is not owned by specified instance"
        );

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
            device: device.id(),
            relevant: Relevant,
            surface: self,
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
        log::warn!(
            "Present mode is not supported. Supported: {:#?}, requested: {:#?}",
            capabilities.present_modes,
            present_mode,
        );
        return Err(SwapchainError::BadPresentMode(present_mode));
    }

    log::trace!(
        "Surface present modes: {:#?}. Pick {:#?}",
        capabilities.present_modes,
        present_mode
    );

    log::trace!("Surface chosen format {:#?}", format);

    assert!(
        capabilities.usage.contains(usage),
        "Surface supports {:?}, but {:?} was requested",
        capabilities.usage,
        usage
    );

    let extent = capabilities.current_extent.unwrap_or(suggest_extent);

    let swap_config = hal::window::SwapchainConfig {
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
    };

    surface
        .raw_mut()
        .configure_swapchain(&device, swap_config)
        .expect("Could not configure swapchain.");

    // let backbuffer = images
    //     .into_iter()
    //     .map(|image| Image {
    //         device: device.id(),
    //         block: None,
    //         relevant: Relevant,
    //         raw: image,
    //         info: ImageInfo {
    //             kind: hal::image::Kind::D2(extent.width, extent.height, 1, 1),
    //             levels: 1,
    //             format,
    //             tiling: hal::image::Tiling::Optimal,
    //             view_caps: hal::image::ViewCapabilities::empty(),
    //             usage,
    //         },
    //     })
    //     .collect();

    Ok(extent)
}

/// Rendering target bound to window.
/// With swapchain created.
pub struct Target<B: Backend> {
    device: DeviceId,
    surface: Surface<B>,
    extent: Extent2D,
    present_mode: hal::window::PresentMode,
    usage: hal::image::Usage,
    relevant: Relevant,
}

device_owned!(Target<B>);

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
    pub unsafe fn dispose(self, device: &Device<B>) -> Surface<B> {
        self.assert_device_owner(device);

        // match self.backbuffer {
        //     Some(images) => {
        //         images
        //             .into_iter()
        //             .for_each(|image| image.dispose_swapchain_image(device.id()));
        //     }
        //     _ => {}
        // };

        self.relevant.dispose();
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
        self.assert_device_owner(device);

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

        let extent = create_swapchain(
            &mut self.surface,
            physical_device,
            device,
            suggest_extent,
            3,
            self.present_mode,
            self.usage,
        )?;

        self.extent = extent;

        Ok(())
    }

    /// Get raw handlers for the swapchain images.
    pub fn backbuffer(&self) -> Vec<Image<B>> {
        Vec::new()
    }

    /// Get render target size.
    pub fn extent(&self) -> Extent2D {
        self.extent
    }

    /// Get image usage flags.
    pub fn usage(&self) -> hal::image::Usage {
        self.usage
    }

    /// Acquire next image.
    pub unsafe fn next_image(
        &mut self,
        _signal: &B::Semaphore,
    ) -> Result<NextImages<'_, B>, AcquireError> {
        let surface_image = match self.surface.raw.acquire_image(!0) {
            Ok((image, _)) => image,
            Err(_) => {
                // FIXME self.recreate_swapchain();
                return Err(AcquireError::OutOfDate);
            }
        };

        Ok(NextImages {
            targets: std::iter::once((&mut *self, surface_image)).collect(),
        })
    }
}

/// Represents acquire frames that will be presented next.
#[derive(Debug)]
pub struct NextImages<'a, B: Backend> {
    targets: smallvec::SmallVec<
        [(
            &'a mut Target<B>,
            <<B as hal::Backend>::Surface as PresentationSurface<B>>::SwapchainImage,
        ); 8],
    >,
}

impl<'a, B> NextImages<'a, B>
where
    B: Backend,
{
    /// Present images by the queue.
    pub unsafe fn present<'b>(
        self,
        queue: &mut impl hal::queue::CommandQueue<B>,
        wait: Option<&B::Semaphore>,
    ) -> Result<Option<hal::window::Suboptimal>, hal::window::PresentError>
    where
        'a: 'b,
    {
        if let Some((target, image)) = self.targets.into_iter().next() {
            queue.present(target.surface.raw_mut(), image, wait)
        } else {
            Err(hal::window::PresentError::OutOfDate)
        }
    }
}
