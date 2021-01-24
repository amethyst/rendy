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
        window::{Extent2D, Surface as _, SurfaceCapabilities},
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
    present_mode: hal::window::PresentMode,
    usage: hal::image::Usage,
) -> Result<(B::Swapchain, Vec<Image<B>>, Extent2D), SwapchainError> {
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

    let (swapchain, images) = device
        .create_swapchain(
            &mut surface.raw,
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
            None,
        )
        .map_err(SwapchainError::Create)?;

    let backbuffer = images
        .into_iter()
        .map(|image| {
            Image::create_from_swapchain(
                device.id(),
                ImageInfo {
                    kind: hal::image::Kind::D2(extent.width, extent.height, 1, 1),
                    levels: 1,
                    format,
                    tiling: hal::image::Tiling::Optimal,
                    view_caps: hal::image::ViewCapabilities::empty(),
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
    surface: Surface<B>,
    swapchain: Option<B::Swapchain>,
    backbuffer: Option<Vec<Image<B>>>,
    extent: Extent2D,
    present_mode: hal::window::PresentMode,
    usage: hal::image::Usage,
    relevant: relevant::Relevant,
}

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
        if let Some(images) = self.backbuffer {
            images
                .into_iter()
                .for_each(|image| image.dispose_swapchain_image(device.id()));
        }

        self.relevant.dispose();
        if let Some(s) = self.swapchain.take() {
            device.destroy_swapchain(s)
        }
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

        if let Some(s) = self.swapchain.take() {
            device.destroy_swapchain(s)
        }

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
    pub unsafe fn swapchain_mut(&mut self) -> &mut impl hal::window::Swapchain<B> {
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
    pub fn usage(&self) -> hal::image::Usage {
        self.usage
    }

    /// Acquire next image.
    pub unsafe fn next_image(
        &mut self,
        signal: &B::Semaphore,
    ) -> Result<NextImages<'_, B>, hal::window::AcquireError> {
        let index = hal::window::Swapchain::acquire_image(
            // Missing swapchain is equivalent to OutOfDate, as it has to be recreated anyway.
            self.swapchain
                .as_mut()
                .ok_or(hal::window::AcquireError::OutOfDate)?,
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
        queue: &mut impl hal::queue::CommandQueue<B>,
        wait: impl IntoIterator<Item = &'b (impl std::borrow::Borrow<B::Semaphore> + 'b)>,
    ) -> Result<Option<hal::window::Suboptimal>, hal::window::PresentError>
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
