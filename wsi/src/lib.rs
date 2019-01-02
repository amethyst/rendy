
//! Window system integration.

#![forbid(overflowing_literals)]
#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]
#![deny(path_statements)]
#![deny(trivial_bounds)]
#![deny(type_alias_bounds)]
#![deny(unconditional_recursion)]
#![deny(unions_with_drop_fields)]
#![deny(while_true)]
#![deny(unused)]
#![deny(bad_style)]
#![deny(future_incompatible)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![allow(unused_unsafe)]

#[cfg(feature = "empty")]
mod gfx_backend_empty {
    pub(super) fn create_surface(instance: &gfx_backend_empty::Instance, window: &winit::Window) -> gfx_backend_empty::Surface {
        unimplemented!()
    }
}

#[cfg(feature = "metal")]
mod gfx_backend_metal {
    pub(super) fn create_surface(instance: &gfx_backend_metal::Instance, window: &winit::Window) -> gfx_backend_metal::Surface {
        instance.create_surface(window)
    }
}

#[cfg(feature = "vulkan")]
mod gfx_backend_vulkan {
    pub(super) fn create_surface(instance: &gfx_backend_vulkan::Instance, window: &winit::Window) -> <gfx_backend_vulkan::Backend as gfx_hal::Backend>::Surface {
        instance.create_surface(window)
    }
}

#[cfg(feature = "dx12")]
mod gfx_backend_dx12 {
    pub(super) fn create_surface(instance: &gfx_backend_dx12::Instance, window: &winit::Window) -> <gfx_backend_dx12::Backend as gfx_hal::Backend>::Surface {
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
                    if let Some(instance) = std::any::Any::downcast_ref(&**$instance) {
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
fn create_surface<B: gfx_hal::Backend>(instance: &Box<dyn std::any::Any>, window: &winit::Window) -> B::Surface {
    create_surface_for_backend!(instance, window);
}

/// Rendering target bound to window.
pub struct Surface<B: gfx_hal::Backend> {
    window: winit::Window,
    raw: B::Surface,
}

impl<B> std::fmt::Debug for Surface<B>
where
    B: gfx_hal::Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Target")
            .field("window", &self.window.id())
            .finish()
    }
}

impl<B> Surface<B>
where
    B: gfx_hal::Backend,
{
    /// Create surface for the window.
    pub fn new(
        instance: &Box<dyn std::any::Any>,
        window: winit::Window,
    ) -> Self {
        let raw = create_surface::<B>(instance, &window);
        Surface {
            window,
            raw,
        }
    }

    /// Get surface image kind.
    pub fn kind(&self) -> gfx_hal::image::Kind {
        gfx_hal::Surface::kind(&self.raw)
    }

    /// Cast surface into render target.
    pub fn into_target(
        mut self,
        physical_device: &B::PhysicalDevice,
        device: &impl gfx_hal::Device<B>,
        image_count: u32,
        usage: gfx_hal::image::Usage,
    ) -> Result<Target<B>, failure::Error> {
        let (capabilities, formats, present_modes, _alpha) = gfx_hal::Surface::compatibility(&self.raw, physical_device);

        let present_mode = *present_modes.iter().max_by_key(|mode| match mode {
            gfx_hal::PresentMode::Immediate => 0,
            gfx_hal::PresentMode::Mailbox => 3,
            gfx_hal::PresentMode::Fifo => 2,
            gfx_hal::PresentMode::Relaxed => 1,
        }).unwrap();

        log::info!("Surface present modes: {:#?}. Pick {:#?}", present_modes, present_mode);

        let formats = formats.unwrap();

        let format = *formats.iter().max_by_key(|format| {
            let base = format.base_format();
            let desc = base.0.desc();
            (!desc.is_compressed(), base.1 == gfx_hal::format::ChannelType::Srgb, desc.bits)
        }).unwrap();

        log::info!("Surface formats: {:#?}. Pick {:#?}", formats, format);

        let image_count = image_count
            .min(capabilities.image_count.end)
            .max(capabilities.image_count.start);

        log::info!("Surface capabilities: {:#?}. Pick {} images", capabilities.image_count, image_count);
        assert!(capabilities.usage.contains(usage), "Surface supports {:?}, but {:?} was requested");

        let extent = capabilities.current_extent.unwrap_or({
            let hidpi_factor = self.window.get_hidpi_factor();
            let start = capabilities.extents.start;
            let end = capabilities.extents.end;
            let (window_width, window_height) = self.window
                .get_inner_size()
                .unwrap()
                .to_physical(hidpi_factor)
                .into();
            gfx_hal::window::Extent2D {
                width: end.width.min(start.width.max(window_width)),
                height: end.height.min(start.height.max(window_height)),
            }
        });

        let (swapchain, backbuffer) = unsafe { device.create_swapchain(
            &mut self.raw,
            gfx_hal::SwapchainConfig {
                present_mode,
                format,
                extent,
                image_count,
                image_layers: 1,
                image_usage: usage,
                composite_alpha: gfx_hal::window::CompositeAlpha::Inherit,
            },
            None,
        ) }?;

        Ok(Target {
            relevant: relevant::Relevant,
            window: self.window,
            surface: self.raw,
            swapchain,
            backbuffer,
            format,
            extent,
            usage,
        })
    }
}

/// Rendering target bound to window.
/// With swapchain created.
pub struct Target<B: gfx_hal::Backend> {
    relevant: relevant::Relevant,
    window: winit::Window,
    surface: B::Surface,
    swapchain: B::Swapchain,
    backbuffer: gfx_hal::Backbuffer<B>,
    format: gfx_hal::format::Format,
    extent: gfx_hal::window::Extent2D,
    usage: gfx_hal::image::Usage,
}

impl<B> std::fmt::Debug for Target<B>
where
    B: gfx_hal::Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = fmt.debug_struct("Target");

        debug.field("window", &self.window.id());

        match self.backbuffer {
            gfx_hal::Backbuffer::Images(ref images) => debug.field("images", &images.len()),
            gfx_hal::Backbuffer::Framebuffer(_) => debug.field("framebuffer", &()),
        };

        debug.field("format", &self.format)
          .field("extent", &self.extent)
          .field("usage", &self.usage)
          .finish()
    }
}

impl<B> Target<B>
where
    B: gfx_hal::Backend,
{
    /// Dispose of target.
    ///
    /// # Safety
    ///
    /// Swapchain must be not in use.
    pub unsafe fn dispose(self, device: &impl gfx_hal::Device<B>) -> winit::Window {
        self.relevant.dispose();
        device.destroy_swapchain(self.swapchain);
        drop(self.surface);
        self.window
    }

    /// Get raw surface handle.
    pub fn surface(&self) -> &B::Surface {
        &self.surface
    }

    /// Get raw surface handle.
    pub fn swapchain(&self) -> &B::Swapchain {
        &self.swapchain
    }

    /// Get swapchain impl trait.
    ///
    /// # Safety
    ///
    /// Trait usage should not violate this type valid usage.
    pub unsafe fn swapchain_mut(&mut self) -> &mut impl gfx_hal::Swapchain<B> {
        &mut self.swapchain
    }

    /// Get image kind of the target images.
    pub fn kind(&self) -> gfx_hal::image::Kind {
        gfx_hal::image::Kind::D2(self.extent.width, self.extent.height, 1, 1)
    }

    /// Get target current extent.
    pub fn extent(&self) -> gfx_hal::window::Extent2D {
        self.extent
    }

    /// Get target current format.
    pub fn format(&self) -> gfx_hal::format::Format {
        self.format
    }

    /// Get raw handlers for the swapchain images.
    pub fn backbuffer(&self) -> &gfx_hal::Backbuffer<B> {
        &self.backbuffer
    }

    /// Acquire next image.
    pub unsafe fn next_image(&mut self, signal: &B::Semaphore) -> Result<NextImages<'_, B>, gfx_hal::AcquireError> {
        let index = gfx_hal::Swapchain::acquire_image(&mut self.swapchain, !0, gfx_hal::FrameSync::Semaphore(signal))?;

        Ok(NextImages {
            targets: std::iter::once((&*self, index)).collect(),
        })
    }
}

/// Represents acquire frames that will be presented next.
#[derive(Debug)]
pub struct NextImages<'a, B: gfx_hal::Backend> {
    targets: smallvec::SmallVec<[(&'a Target<B>, u32); 8]>,
}

impl<'a, B> NextImages<'a, B>
where
    B: gfx_hal::Backend,
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
        wait: impl IntoIterator<Item = &'b (impl std::borrow::Borrow<B::Semaphore> + 'b)>
    ) -> Result<(), failure::Error>
    where
        'a: 'b
    {
        queue.present(
            self.targets.iter().map(|(target, index)| (&target.swapchain, *index)),
            wait,
        ).map_err(|()| failure::format_err!("Suboptimal or out of date?"))
    }
}

impl<'a, B> std::ops::Index<usize> for NextImages<'a, B>
where
    B: gfx_hal::Backend,
{
    type Output = u32;

    fn index(&self, index: usize) -> &u32 {
        &self.targets[index].1
    }
}
