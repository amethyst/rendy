
#[cfg(feature = "gfx-backend-metal")]
mod metal {

fn create_surface(instance: &gfx_backend_metal::Instance, window: &winit::Window) -> gfx_backend_metal::Surface {
    let nsview = winit::os::macos::WindowExt::get_nsview(window);
    instance.create_surface_from_nsview(nsview)
}

}

macro_rules! create_surface_for_backend {
    ($instance:ident, $window:ident | $backend:ident @ $module:ident ? $feature:meta) => {
        #[$feature]
        {
            if let Some(instance) = std::any::Any::downcast_ref::<$backend::Instance>($instance) {
                let surface: Box<std::any::Any> = Box::new(self::$module::create_surface(instance, $window));
                let surface = Box::downcast::<B::Surface>(surface).expect(concat!("`", stringify!($backend), "::Backend::Surface` must be `", stringify!($backend), "::Surface`"));
                return *surface;
            }
        }
    };

    ($instance:ident, $window:ident $(| $backend:ident @ $module:ident ? $feature:meta)*) => {
        $(create_surface_for_backend!($instance, $window | $backend @ $module ? $feature));*
    };

    ($instance:ident, $window:ident) => {
        create_surface_for_backend!($instance, $window
            | gfx_backend_dx12 @ dx12 ? cfg(feature = "gfx-backend-dx12")
            | gfx_backend_metal @ metal ? cfg(feature = "gfx-backend-metal")
            | gfx_backend_vulkan @ vulkan ? cfg(feature = "gfx-backend-vulkan")
        );
    };
}

fn create_surface<B: gfx_hal::Backend>(instance: &dyn gfx_hal::Instance<Backend = B>, window: &winit::Window) -> B::Surface {
    create_surface_for_backend!(instance, window);
    unreachable!()
}

/// Rendering target bound to window.
pub struct Target<B: gfx_hal::Backend> {
    window: winit::Window,
    surface: B::Surface,
    swapchain: B::Swapchain,
    images: Vec<B::Image>,
    format: gfx_hal::format::Format,
    extent: gfx_hal::window::Extent2D,
    relevant: relevant::Relevant,
}

impl<B> Target<B>
where
    B: gfx_hal::Backend,
{
    pub fn new(
        instance: &dyn gfx_hal::Instance<Backend = B>,
        physical_device: &B::PhysicalDevice,
        device: &impl gfx_hal::Device<B>,
        window: winit::Window,
        image_count: u32,
        usage: gfx_hal::image::Usage,
    ) -> Result<Self, failure::Error> {
        let mut surface = create_surface(instance, &window);

        let (capabilities, formats, present_modes) = gfx_hal::Surface::compatibility(&surface, physical_device);

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
            (!desc.is_compressed(), desc.bits, base.1 == gfx_hal::format::ChannelType::Srgb)
        }).unwrap();

        log::info!("Surface formats: {:#?}. Pick {:#?}", formats, format);

        let image_count = image_count
            .min(capabilities.image_count.end)
            .max(capabilities.image_count.start);

        log::info!("Surface capabilities: {:#?}. Pick {} images", capabilities.image_count, image_count);
        assert!(capabilities.usage.contains(usage));

        let (swapchain, backbuffer) = device.create_swapchain(
            &mut surface,
            gfx_hal::SwapchainConfig {
                present_mode,
                format,
                extent: capabilities.current_extent.unwrap(),
                image_count,
                image_layers: 1,
                image_usage: usage,
            },
            None,
        )?;

        let images = if let gfx_hal::Backbuffer::Images(images) = backbuffer {
            images
        } else {
            panic!("Framebuffer backbuffer is not supported");
        };

        Ok(Target {
            window,
            surface,
            swapchain,
            images,
            format,
            extent: capabilities.current_extent.unwrap(),
            relevant: relevant::Relevant,
        })
    }

    /// Strip the target to the internal parts.
    ///
    /// # Safety
    ///
    /// Swapchain must be not in use.
    pub unsafe fn dispose(self, device: &impl gfx_hal::Device<B>) -> winit::Window {
        device.destroy_swapchain(self.swapchain);
        drop(self.surface);
        self.relevant.dispose();
        self.window
    }

    /// Get raw surface handle.
    ///
    /// # Safety
    ///
    /// Trait usage should not violate this type valid usage.
    pub unsafe fn surface(&self) -> &impl gfx_hal::Surface<B> {
        &self.surface
    }

    /// Get raw surface handle.
    ///
    /// # Safety
    ///
    /// Trait usage should not violate this type valid usage.
    pub unsafe fn swapchain(&self) -> &impl gfx_hal::Swapchain<B> {
        &self.swapchain
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
    pub fn images(&self) -> &[B::Image] {
        &self.images
    }

    /// Acquire next image.
    pub fn next_image(&mut self, signal: &B::Semaphore) -> Result<NextImages<'_, B>, gfx_hal::AcquireError> {
        let index = unsafe {
            gfx_hal::Swapchain::acquire_image(&mut self.swapchain, !0, gfx_hal::FrameSync::Semaphore(signal))
        }?;

        Ok(NextImages {
            swapchains: std::iter::once((&self.swapchain, index)).collect(),
        })
    }
}

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct NextImages<'a, B: gfx_hal::Backend> {
    #[derivative(Debug = "ignore")]
    swapchains: smallvec::SmallVec<[(&'a B::Swapchain, u32); 8]>,
}

impl<'a, B> NextImages<'a, B>
where
    B: gfx_hal::Backend,
{
    /// Get indices.
    pub fn indices(&self) -> impl IntoIterator<Item = u32> + '_ {
        self.swapchains.iter().map(|(s, i)| *i)
    }

    /// Present images by the queue.
    /// 
    /// # TODO
    /// 
    /// Use specific presentation error type.
    pub fn present(self, queue: &mut impl gfx_hal::queue::RawCommandQueue<B>, wait: &[B::Semaphore]) -> Result<(), failure::Error> {
        unsafe {
            queue.present(
                self.swapchains.iter().cloned(),
                wait,
            ).map_err(|()| failure::format_err!("Suboptimal or out of date?"))
        }
    }
}
