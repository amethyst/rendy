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
    rendy_core::hal::{device::Device as _, window::Extent2D, Backend},
    rendy_core::{
        device_owned, instance_owned, rendy_with_dx12_backend, rendy_with_empty_backend,
        rendy_with_metal_backend, rendy_with_vulkan_backend, Device, DeviceId, Instance,
        InstanceId,
    },
    rendy_resource::{Image, ImageInfo},
};

#[cfg(feature = "winit")]
use rendy_core::rendy_backend_match;

#[cfg(feature = "winit")]
pub use winit;

rendy_with_empty_backend! {
    mod gfx_backend_empty {
        #[cfg(feature = "winit")]
        pub(super) fn create_surface(
            _instance: &rendy_core::empty::Instance,
            _window: &winit::window::Window,
        ) -> rendy_core::empty::Surface {
            rendy_core::empty::Surface
        }
    }
}

rendy_with_dx12_backend! {
    mod gfx_backend_dx12 {
        #[cfg(feature = "winit")]
        pub(super) fn create_surface(
            instance: &rendy_core::dx12::Instance,
            window: &winit::window::Window,
        ) -> <rendy_core::dx12::Backend as rendy_core::hal::Backend>::Surface {
            instance.create_surface(window)
        }
    }
}

rendy_with_metal_backend! {
    mod gfx_backend_metal {
        #[cfg(feature = "winit")]
        pub(super) fn create_surface(
            instance: &rendy_core::metal::Instance,
            window: &winit::window::Window,
        ) -> <rendy_core::metal::Backend as rendy_core::hal::Backend>::Surface {
            instance.create_surface(window)
        }
    }
}

rendy_with_vulkan_backend! {
    mod gfx_backend_vulkan {
        #[cfg(feature = "winit")]
        pub(super) fn create_surface(
            instance: &rendy_core::vulkan::Instance,
            window: &winit::window::Window,
        ) -> <rendy_core::vulkan::Backend as rendy_core::hal::Backend>::Surface {
            instance.create_surface(window)
        }
    }
}

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

#[cfg(feature = "winit")]
#[allow(unused)]
fn create_surface<B: Backend>(
    instance: &Instance<B>,
    window: &winit::window::Window,
) -> B::Surface {
    use rendy_core::identical_cast;

    // We perform identical type transmute.
    rendy_backend_match!(B {
        empty => {
            identical_cast(gfx_backend_empty::create_surface(instance.raw_typed().unwrap(), window))
        }
        dx12 => {
            identical_cast(gfx_backend_dx12::create_surface(instance.raw_typed().unwrap(), window))
        }
        metal => {
            identical_cast(gfx_backend_metal::create_surface(instance.raw_typed().unwrap(), window))
        }
        vulkan => {
            identical_cast(gfx_backend_vulkan::create_surface(instance.raw_typed().unwrap(), window))
        }
    })
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
    #[cfg(feature = "winit")]
    pub fn new(instance: &Instance<B>, window: &winit::window::Window) -> Self {
        let raw = create_surface::<B>(instance, &window);
        Surface {
            raw,
            instance: instance.id(),
        }
    }

    /// Create surface from `instance`.
    ///
    /// # Safety
    ///
    /// Closure must return surface object created from raw instance provided as closure argument.
    pub unsafe fn create<T>(instance: &Instance<B>, f: impl FnOnce(&T) -> B::Surface) -> Self
    where
        T: rendy_core::hal::Instance<Backend = B>,
    {
        Surface {
            raw: f(instance.raw_typed().expect("Wrong instance type")),
            instance: instance.id(),
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
        let (capabilities, _formats, _present_modes) = self.compatibility(physical_device);
        capabilities.current_extent
    }

    /// Get surface ideal format.
    pub unsafe fn format(
        &self,
        physical_device: &B::PhysicalDevice,
    ) -> rendy_core::hal::format::Format {
        let (_capabilities, formats, _present_modes) =
            rendy_core::hal::window::Surface::compatibility(&self.raw, physical_device);
        let formats = formats.unwrap();

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
    }

    /// Get surface compatibility
    ///
    /// ## Safety
    /// - `physical_device` must be created from same `Instance` as the `Surface`
    pub unsafe fn compatibility(
        &self,
        physical_device: &B::PhysicalDevice,
    ) -> (
        rendy_core::hal::window::SurfaceCapabilities,
        Option<Vec<rendy_core::hal::format::Format>>,
        Vec<rendy_core::hal::window::PresentMode>,
    ) {
        rendy_core::hal::window::Surface::compatibility(&self.raw, physical_device)
    }

    /// Cast surface into render target.
    pub unsafe fn into_target(
        mut self,
        physical_device: &B::PhysicalDevice,
        device: &Device<B>,
        suggest_extent: Extent2D,
        image_count: u32,
        present_mode: rendy_core::hal::window::PresentMode,
        usage: rendy_core::hal::image::Usage,
    ) -> Result<Target<B>, SwapchainError> {
        assert_eq!(
            device.id().instance,
            self.instance,
            "Resource is not owned by specified instance"
        );

        let (swapchain, backbuffer, extent) = create_swapchain(
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
            relevant: relevant::Relevant,
            surface: self,
            swapchain: Some(swapchain),
            backbuffer: Some(backbuffer),
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
    present_mode: rendy_core::hal::window::PresentMode,
    usage: rendy_core::hal::image::Usage,
) -> Result<(B::Swapchain, Vec<Image<B>>, Extent2D), SwapchainError> {
    let (capabilities, formats, present_modes) = surface.compatibility(physical_device);

    if !present_modes.contains(&present_mode) {
        log::warn!(
            "Present mode is not supported. Supported: {:#?}, requested: {:#?}",
            present_modes,
            present_mode,
        );
        return Err(SwapchainError::BadPresentMode(present_mode));
    }

    log::trace!(
        "Surface present modes: {:#?}. Pick {:#?}",
        present_modes,
        present_mode
    );

    let formats = formats.unwrap();

    let format = *formats
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
        .unwrap();

    log::trace!("Surface formats: {:#?}. Pick {:#?}", formats, format);

    if image_count < *capabilities.image_count.start()
        || image_count > *capabilities.image_count.end()
    {
        log::warn!(
            "Image count not supported. Supported: {:#?}, requested: {:#?}",
            capabilities.image_count,
            image_count
        );
        return Err(SwapchainError::BadImageCount(image_count));
    }

    log::trace!(
        "Surface capabilities: {:#?}. Pick {} images",
        capabilities.image_count,
        image_count
    );

    assert!(
        capabilities.usage.contains(usage),
        "Surface supports {:?}, but {:?} was requested",
        capabilities.usage,
        usage
    );

    let extent = capabilities.current_extent.unwrap_or(suggest_extent);

    let (swapchain, images) = device
        .create_swapchain(
            &mut surface.raw,
            rendy_core::hal::window::SwapchainConfig {
                present_mode,
                format,
                extent,
                image_count,
                image_layers: 1,
                image_usage: usage,
                composite_alpha: [
                    rendy_core::hal::window::CompositeAlpha::INHERIT,
                    rendy_core::hal::window::CompositeAlpha::OPAQUE,
                    rendy_core::hal::window::CompositeAlpha::PREMULTIPLIED,
                    rendy_core::hal::window::CompositeAlpha::POSTMULTIPLIED,
                ]
                .iter()
                .find(|&bit| capabilities.composite_alpha & *bit == *bit)
                .cloned()
                .expect("No CompositeAlpha modes supported"),
            },
            None,
        )
        .map_err(SwapchainError::Create)?;

    let backbuffer = images
        .into_iter()
        .map(|image| {
            Image::create_from_swapchain(
                device.id(),
                ImageInfo {
                    kind: rendy_core::hal::image::Kind::D2(extent.width, extent.height, 1, 1),
                    levels: 1,
                    format,
                    tiling: rendy_core::hal::image::Tiling::Optimal,
                    view_caps: rendy_core::hal::image::ViewCapabilities::empty(),
                    usage,
                },
                image,
            )
        })
        .collect();

    Ok((swapchain, backbuffer, extent))
}

/// Rendering target bound to window.
/// With swapchain created.
pub struct Target<B: Backend> {
    device: DeviceId,
    surface: Surface<B>,
    swapchain: Option<B::Swapchain>,
    backbuffer: Option<Vec<Image<B>>>,
    extent: Extent2D,
    present_mode: rendy_core::hal::window::PresentMode,
    usage: rendy_core::hal::image::Usage,
    relevant: relevant::Relevant,
}

device_owned!(Target<B>);

impl<B> std::fmt::Debug for Target<B>
where
    B: Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Target")
            .field("backbuffer", &self.backbuffer)
            .finish()
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
        self.assert_device_owner(device);

        match self.backbuffer {
            Some(images) => {
                images
                    .into_iter()
                    .for_each(|image| image.dispose_swapchain_image(device.id()));
            }
            _ => {}
        };

        self.relevant.dispose();
        self.swapchain.take().map(|s| device.destroy_swapchain(s));
        self.surface
    }

    /// Get raw surface handle.
    pub fn surface(&self) -> &Surface<B> {
        &self.surface
    }

    /// Get raw surface handle.
    pub fn swapchain(&self) -> &B::Swapchain {
        self.swapchain.as_ref().expect("Swapchain already disposed")
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

        let image_count = match self.backbuffer.take() {
            Some(images) => {
                let count = images.len();
                images
                    .into_iter()
                    .for_each(|image| image.dispose_swapchain_image(device.id()));
                count
            }
            None => 0,
        };

        self.swapchain.take().map(|s| device.destroy_swapchain(s));

        let (swapchain, backbuffer, extent) = create_swapchain(
            &mut self.surface,
            physical_device,
            device,
            suggest_extent,
            image_count as u32,
            self.present_mode,
            self.usage,
        )?;

        self.swapchain.replace(swapchain);
        self.backbuffer.replace(backbuffer);
        self.extent = extent;

        Ok(())
    }

    /// Get swapchain impl trait.
    ///
    /// # Safety
    ///
    /// Trait usage should not violate this type valid usage.
    pub unsafe fn swapchain_mut(&mut self) -> &mut impl rendy_core::hal::window::Swapchain<B> {
        self.swapchain.as_mut().expect("Swapchain already disposed")
    }

    /// Get raw handlers for the swapchain images.
    pub fn backbuffer(&self) -> &Vec<Image<B>> {
        self.backbuffer
            .as_ref()
            .expect("Swapchain already disposed")
    }

    /// Get render target size.
    pub fn extent(&self) -> Extent2D {
        self.extent
    }

    /// Get image usage flags.
    pub fn usage(&self) -> rendy_core::hal::image::Usage {
        self.usage
    }

    /// Acquire next image.
    pub unsafe fn next_image(
        &mut self,
        signal: &B::Semaphore,
    ) -> Result<NextImages<'_, B>, rendy_core::hal::window::AcquireError> {
        let index = rendy_core::hal::window::Swapchain::acquire_image(
            // Missing swapchain is equivalent to OutOfDate, as it has to be recreated anyway.
            self.swapchain
                .as_mut()
                .ok_or(rendy_core::hal::window::AcquireError::OutOfDate)?,
            !0,
            Some(signal),
            None,
        )?
        .0;

        Ok(NextImages {
            targets: std::iter::once((&*self, index)).collect(),
        })
    }
}

/// Represents acquire frames that will be presented next.
#[derive(Debug)]
pub struct NextImages<'a, B: Backend> {
    targets: smallvec::SmallVec<[(&'a Target<B>, u32); 8]>,
}

impl<'a, B> NextImages<'a, B>
where
    B: Backend,
{
    /// Get indices.
    pub fn indices(&self) -> impl IntoIterator<Item = u32> + '_ {
        self.targets.iter().map(|(_s, i)| *i)
    }

    /// Present images by the queue.
    ///
    /// # TODO
    ///
    /// Use specific presentation error type.
    pub unsafe fn present<'b>(
        self,
        queue: &mut impl rendy_core::hal::queue::CommandQueue<B>,
        wait: impl IntoIterator<Item = &'b (impl std::borrow::Borrow<B::Semaphore> + 'b)>,
    ) -> Result<Option<rendy_core::hal::window::Suboptimal>, rendy_core::hal::window::PresentError>
    where
        'a: 'b,
    {
        queue.present(
            self.targets.iter().map(|(target, index)| {
                (
                    target
                        .swapchain
                        .as_ref()
                        .expect("Swapchain already disposed"),
                    *index,
                )
            }),
            wait,
        )
    }
}

impl<'a, B> std::ops::Index<usize> for NextImages<'a, B>
where
    B: Backend,
{
    type Output = u32;

    fn index(&self, index: usize) -> &u32 {
        &self.targets[index].1
    }
}

/// Resolve into input AST if winit support is enabled.
#[cfg(feature = "winit")]
#[macro_export]
macro_rules! with_winit {
    ($($t:tt)*) => { $($t)* };
}

/// Resolve into input AST if winit support is enabled.
#[cfg(not(feature = "winit"))]
#[macro_export]
macro_rules! with_winit {
    ($($t:tt)*) => {};
}

/// Resolve into input AST if winit support is disabled.
#[cfg(not(feature = "winit"))]
#[macro_export]
macro_rules! without_winit {
    ($($t:tt)*) => { $($t)* };
}

/// Resolve into input AST if winit support is disabled.
#[cfg(feature = "winit")]
#[macro_export]
macro_rules! without_winit {
    ($($t:tt)*) => {};
}
