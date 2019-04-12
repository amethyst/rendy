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
    gfx_hal::{Backend, Device as _},
    rendy_resource::{Image, ImageInfo},
    rendy_util::{device_owned, instance_owned, Device, DeviceId, Instance, InstanceId},
};

#[cfg(feature = "empty")]
mod gfx_backend_empty {
    pub(super) fn create_surface(
        _instance: &gfx_backend_empty::Instance,
        _window: &winit::Window,
    ) -> <gfx_backend_empty::Backend as gfx_hal::Backend>::Surface {
        unimplemented!()
    }
}

#[cfg(feature = "metal")]
mod gfx_backend_metal {
    pub(super) fn create_surface(
        instance: &gfx_backend_metal::Instance,
        window: &winit::Window,
    ) -> <gfx_backend_metal::Backend as gfx_hal::Backend>::Surface {
        instance.create_surface(window)
    }
}

#[cfg(feature = "vulkan")]
mod gfx_backend_vulkan {
    pub(super) fn create_surface(
        instance: &gfx_backend_vulkan::Instance,
        window: &winit::Window,
    ) -> <gfx_backend_vulkan::Backend as gfx_hal::Backend>::Surface {
        instance.create_surface(window)
    }
}

#[cfg(feature = "dx12")]
mod gfx_backend_dx12 {
    pub(super) fn create_surface(
        instance: &gfx_backend_dx12::Instance,
        window: &winit::Window,
    ) -> <gfx_backend_dx12::Backend as gfx_hal::Backend>::Surface {
        instance.create_surface(window)
    }
}

macro_rules! create_surface_for_backend {
    (match $instance:ident, $window:ident $(| $backend:ident @ $feature:meta)+) => {{
        #[allow(non_camel_case_types)]
        enum _B {$(
            $backend,
        )+}

        for b in [$(_B::$backend),+].iter() {
            match b {$(
                #[$feature]
                _B::$backend => {
                    if let Some(instance) = $instance.raw_typed() {
                        let surface: Box<dyn std::any::Any> = Box::new(self::$backend::create_surface(instance, $window));
                        return *surface.downcast().expect(concat!("`", stringify!($backend), "::Backend::Surface` must be `", stringify!($backend), "::Surface`"));
                    }
                })+
                _ => continue,
            }
        }
        panic!("
            Undefined backend requested.
            Make sure feature for required backend is enabled.
            Try to add `--features=vulkan` or if on macos `--features=metal`.
        ")
    }};

    ($instance:ident, $window:ident) => {{
        create_surface_for_backend!(match $instance, $window
            | gfx_backend_empty @ cfg(feature = "empty")
            | gfx_backend_dx12 @ cfg(feature = "dx12")
            | gfx_backend_metal @ cfg(feature = "metal")
            | gfx_backend_vulkan @ cfg(feature = "vulkan")
        );
    }};
}

#[allow(unused_variables)]
fn create_surface<B: Backend>(instance: &Instance<B>, window: &winit::Window) -> B::Surface {
    create_surface_for_backend!(instance, window);
}

/// Rendering target bound to window.
pub struct Surface<B: Backend> {
    window: std::sync::Arc<winit::Window>,
    raw: B::Surface,
    instance: InstanceId,
}

impl<B> std::fmt::Debug for Surface<B>
where
    B: Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Target")
            .field("window", &self.window.id())
            .finish()
    }
}

instance_owned!(Surface<B>);

impl<B> Surface<B>
where
    B: Backend,
{
    /// Create surface for the window.
    pub fn new(instance: &Instance<B>, window: std::sync::Arc<winit::Window>) -> Self {
        let raw = create_surface::<B>(instance, &window);
        Surface {
            window,
            raw,
            instance: instance.id(),
        }
    }

    /// Get raw `B::Surface` reference
    pub fn raw(&self) -> &B::Surface {
        &self.raw
    }

    /// Get surface image kind.
    pub fn kind(&self) -> gfx_hal::image::Kind {
        gfx_hal::Surface::kind(&self.raw)
    }

    /// Get width to hight ratio.
    pub fn aspect(&self) -> f32 {
        let extent = gfx_hal::Surface::kind(&self.raw).extent();
        extent.width as f32 / extent.height as f32
    }

    /// Get surface ideal format.
    pub unsafe fn format(&self, physical_device: &B::PhysicalDevice) -> gfx_hal::format::Format {
        let (_capabilities, formats, _present_modes, _alpha) =
            gfx_hal::Surface::compatibility(&self.raw, physical_device);
        let formats = formats.unwrap();

        *formats
            .iter()
            .max_by_key(|format| {
                let base = format.base_format();
                let desc = base.0.desc();
                (
                    !desc.is_compressed(),
                    base.1 == gfx_hal::format::ChannelType::Srgb,
                    desc.bits,
                )
            })
            .unwrap()
    }

    /// Get surface compatibility
    ///
    /// ## Safety
    /// - `physical_device` must be created from same `Instance` as the `Surface`
    pub unsafe fn compatibility(
        &self,
        physical_device: &B::PhysicalDevice,
    ) -> (
        gfx_hal::window::SurfaceCapabilities,
        Option<Vec<gfx_hal::format::Format>>,
        Vec<gfx_hal::PresentMode>,
        Vec<gfx_hal::CompositeAlpha>,
    ) {
        gfx_hal::Surface::compatibility(&self.raw, physical_device)
    }

    /// Cast surface into render target.
    pub unsafe fn into_target(
        mut self,
        physical_device: &B::PhysicalDevice,
        device: &Device<B>,
        image_count: u32,
        present_mode: gfx_hal::PresentMode,
        usage: gfx_hal::image::Usage,
    ) -> Result<Target<B>, failure::Error> {
        assert_eq!(
            device.id().instance,
            self.instance,
            "Resource is not owned by specified instance"
        );

        let (swapchain, backbuffer) = create_swapchain(
            &mut self,
            physical_device,
            device,
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
            present_mode,
            usage,
        })
    }

    /// Get a reference to the internal window.
    pub fn window(&self) -> &winit::Window {
        &self.window
    }
}

unsafe fn create_swapchain<B: Backend>(
    surface: &mut Surface<B>,
    physical_device: &B::PhysicalDevice,
    device: &Device<B>,
    image_count: u32,
    present_mode: gfx_hal::PresentMode,
    usage: gfx_hal::image::Usage,
) -> Result<(B::Swapchain, Backbuffer<B>), failure::Error> {
    let (capabilities, formats, present_modes, alpha) = surface.compatibility(physical_device);

    if !present_modes.contains(&present_mode) {
        log::warn!(
            "Present mode is not supported. Supported: {:#?}, requested: {:#?}",
            present_modes,
            present_mode,
        );
        failure::bail!("Present mode not supported.");
    }

    log::info!(
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
                base.1 == gfx_hal::format::ChannelType::Srgb,
                desc.bits,
            )
        })
        .unwrap();

    log::info!("Surface formats: {:#?}. Pick {:#?}", formats, format);

    if image_count < capabilities.image_count.start || image_count > capabilities.image_count.end {
        log::warn!(
            "Image count not supported. Supported: {:#?}, requested: {:#?}",
            capabilities.image_count,
            image_count
        );
        failure::bail!("Image count not supported.")
    }

    log::info!(
        "Surface capabilities: {:#?}. Pick {} images",
        capabilities.image_count,
        image_count
    );

    assert!(
        capabilities.usage.contains(usage),
        "Surface supports {:?}, but {:?} was requested"
    );

    let extent = capabilities.current_extent.unwrap_or({
        let hidpi_factor = surface.window.get_hidpi_factor();
        let start = capabilities.extents.start;
        let end = capabilities.extents.end;
        let (window_width, window_height) = surface
            .window
            .get_inner_size()
            .unwrap()
            .to_physical(hidpi_factor)
            .into();
        gfx_hal::window::Extent2D {
            width: end.width.min(start.width.max(window_width)),
            height: end.height.min(start.height.max(window_height)),
        }
    });

    let (swapchain, backbuffer) = device.create_swapchain(
        &mut surface.raw,
        gfx_hal::SwapchainConfig {
            present_mode,
            format,
            extent,
            image_count,
            image_layers: 1,
            image_usage: usage,
            composite_alpha: alpha
                .into_iter()
                .max_by_key(|alpha| match alpha {
                    gfx_hal::window::CompositeAlpha::Inherit => 3,
                    gfx_hal::window::CompositeAlpha::Opaque => 2,
                    gfx_hal::window::CompositeAlpha::PreMultiplied => 1,
                    gfx_hal::window::CompositeAlpha::PostMultiplied => 0,
                })
                .expect("No CompositeAlpha modes supported"),
        },
        None,
    )?;

    let backbuffer = match backbuffer {
        gfx_hal::Backbuffer::Images(images) => {
            let backbuffer = Backbuffer::Images(
                images
                    .into_iter()
                    .map(|image| {
                        Image::create_from_swapchain(
                            device.id(),
                            ImageInfo {
                                kind: gfx_hal::image::Kind::D2(extent.width, extent.height, 1, 1),
                                levels: 1,
                                format,
                                tiling: gfx_hal::image::Tiling::Optimal,
                                view_caps: gfx_hal::image::ViewCapabilities::empty(),
                                usage,
                            },
                            image,
                        )
                    })
                    .collect(),
            );
            backbuffer
        }
        gfx_hal::Backbuffer::Framebuffer(raw) => {
            let backbuffer = Backbuffer::Framebuffer {
                raw,
                format,
                extent,
            };
            backbuffer
        }
    };

    Ok((swapchain, backbuffer))
}

/// Backbuffer of the `Target`.
/// Either collection of `Image`s
/// or framebuffer.
#[derive(Debug)]
pub enum Backbuffer<B: Backend> {
    /// Collection of images that in the `Target`'s swapchain.
    Images(Vec<Image<B>>),

    /// Framebuffer of the `Target`.
    Framebuffer {
        /// Raw framebuffer.
        /// Can be used with any render-pass.
        raw: B::Framebuffer,

        /// Formats of image in framebuffer.
        format: gfx_hal::format::Format,

        /// Extent of image in framebuffer.
        extent: gfx_hal::window::Extent2D,
    },
}

/// Rendering target bound to window.
/// With swapchain created.
pub struct Target<B: Backend> {
    device: DeviceId,
    surface: Surface<B>,
    swapchain: Option<B::Swapchain>,
    backbuffer: Option<Backbuffer<B>>,
    present_mode: gfx_hal::PresentMode,
    usage: gfx_hal::image::Usage,
    relevant: relevant::Relevant,
}

device_owned!(Target<B>);

impl<B> std::fmt::Debug for Target<B>
where
    B: Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Target")
            .field("window", &self.surface.window().id())
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
            Some(Backbuffer::Images(images)) => {
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
    /// Current swapchain must be not in use.
    pub unsafe fn recreate(
        &mut self,
        physical_device: &B::PhysicalDevice,
        device: &Device<B>,
    ) -> Result<(), failure::Error> {
        self.assert_device_owner(device);

        let image_count = match self.backbuffer.take() {
            Some(Backbuffer::Images(images)) => {
                let count = images.len();
                images
                    .into_iter()
                    .for_each(|image| image.dispose_swapchain_image(device.id()));
                count
            }
            Some(Backbuffer::Framebuffer { .. }) => 0,
            None => 0,
        };

        self.swapchain.take().map(|s| device.destroy_swapchain(s));

        let (swapchain, backbuffer) = create_swapchain(
            &mut self.surface,
            physical_device,
            device,
            image_count as u32,
            self.present_mode,
            self.usage,
        )?;

        self.swapchain.replace(swapchain);
        self.backbuffer.replace(backbuffer);

        Ok(())
    }

    /// Get swapchain impl trait.
    ///
    /// # Safety
    ///
    /// Trait usage should not violate this type valid usage.
    pub unsafe fn swapchain_mut(&mut self) -> &mut impl gfx_hal::Swapchain<B> {
        self.swapchain.as_mut().expect("Swapchain already disposed")
    }

    /// Get raw handlers for the swapchain images or framebuffer.
    pub fn backbuffer(&self) -> &Backbuffer<B> {
        self.backbuffer
            .as_ref()
            .expect("Swapchain already disposed")
    }

    /// Acquire next image.
    pub unsafe fn next_image(
        &mut self,
        signal: &B::Semaphore,
    ) -> Result<NextImages<'_, B>, gfx_hal::AcquireError> {
        let index = gfx_hal::Swapchain::acquire_image(
            // Missing swapchain is equivalent to OutOfDate, as it has to be recreated anyway.
            self.swapchain
                .as_mut()
                .ok_or(gfx_hal::AcquireError::OutOfDate)?,
            !0,
            gfx_hal::FrameSync::Semaphore(signal),
        )?;

        Ok(NextImages {
            targets: std::iter::once((&*self, index)).collect(),
        })
    }

    /// Get a reference to the internal window.
    pub fn window(&self) -> &winit::Window {
        &self.surface.window()
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
        queue: &mut impl gfx_hal::queue::RawCommandQueue<B>,
        wait: impl IntoIterator<Item = &'b (impl std::borrow::Borrow<B::Semaphore> + 'b)>,
    ) -> Result<(), failure::Error>
    where
        'a: 'b,
    {
        queue
            .present(
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
            .map_err(|()| failure::format_err!("Suboptimal or out of date, or what?"))
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
