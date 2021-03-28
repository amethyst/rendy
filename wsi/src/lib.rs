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
        window::{
            Extent2D, Surface as _, SurfaceCapabilities, SwapchainConfig,
            PresentationSurface, Suboptimal, AcquireError, SwapchainError,
            PresentError,
        },
        queue::CommandQueue,
        Backend, Instance as _,
    },
    rendy_core::{
        device_owned, instance_owned, Device, DeviceId, HasRawWindowHandle, Instance, InstanceId,
    },
    rendy_resource::{Image, ImageInfo},
};

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

    /// Get mutable raw `B::Surface` reference
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

    /// Set up the swapchain associated with the surface to have the given format.
    pub unsafe fn configure_swapchain(
        &mut self,
        device: &B::Device,
        config: SwapchainConfig,
    ) -> Result<(), SwapchainError> {
        self.raw.configure_swapchain(device, config)
    }

    /// Remove the associated swapchain from this surface.
    ///
    /// This has to be done before the surface is dropped.
    pub unsafe fn unconfigure_swapchain(
        &mut self,
        device: &B::Device,
    ) {
        self.raw.unconfigure_swapchain(device)
    }

    /// Acquire a new swapchain image for rendering.
    /// 
    /// May fail according to one of the reasons indicated in AcquireError enum.
    ///
    /// ## Synchronization
    /// The acquired image is available to render. No synchronization is required.
    pub unsafe fn acquire_image(
        &mut self,
        timeout_ns: u64,
    ) -> Result<(<B::Surface as PresentationSurface<B>>::SwapchainImage, Option<Suboptimal>), AcquireError> {
        self.raw.acquire_image(timeout_ns)
    }

    pub unsafe fn present(
        &mut self,
        queue: &mut impl CommandQueue<B>,
        image: <B::Surface as PresentationSurface<B>>::SwapchainImage,
        wait_semaphore: Option<&mut B::Semaphore>,
    ) -> Result<Option<Suboptimal>, PresentError> {
        queue.present(
            &mut self.raw,
            image,
            wait_semaphore,
        )
    }
}
