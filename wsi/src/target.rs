use std::cmp::{max, min};

use ash::{
    extensions::{Surface, Swapchain},
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0},
    vk,
};

use failure::Error;
use relevant::Relevant;
use smallvec::SmallVec;
use winit::Window;

use NativeSurface;

pub struct Target {
    fp: Swapchain,
    window: Window,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    format: vk::Format,
    extent: vk::Extent2D,
    relevant: Relevant,
}

impl Target {
    pub fn new(
        window: Window,
        image_count: u32,
        physical: vk::PhysicalDevice,
        native_surface: &NativeSurface,
        surface: &Surface,
        swapchain: &Swapchain,
    ) -> Result<Self, Error> {
        let surface_khr = native_surface.create_surface(&window)?;

        let present_modes = unsafe {
            surface.get_physical_device_surface_present_modes_khr(physical, surface_khr)
        }?;
        info!("Present modes: {:#?}", present_modes);

        let formats =
            unsafe { surface.get_physical_device_surface_formats_khr(physical, surface_khr) }?;
        info!("Formats: {:#?}", formats);

        let capabilities =
            unsafe { surface.get_physical_device_surface_capabilities_khr(physical, surface_khr) }?;
        info!("Capabilities: {:#?}", capabilities);

        let image_count = max(
            min(image_count, capabilities.max_image_count),
            capabilities.min_image_count,
        );

        let swapchain_khr = unsafe {
            swapchain.create_swapchain_khr(
                &vk::SwapchainCreateInfoKHR::builder()
                    .surface(surface_khr)
                    .min_image_count(image_count)
                    .image_format(formats[0].format)
                    .image_extent(capabilities.current_extent)
                    .image_array_layers(1)
                    .image_usage(capabilities.supported_usage_flags)
                    .present_mode(*present_modes.first().unwrap())
                    .build(),
                None,
            )
        }?;

        let images =
            unsafe { swapchain.get_swapchain_images_khr(swapchain_khr) }.map_err(Error::from)?;

        // trace!("Target created");

        Ok(Target {
            fp: swapchain.clone(),
            window,
            surface: surface_khr,
            swapchain: swapchain_khr,
            images,
            format: formats[0].format,
            extent: capabilities.current_extent,
            relevant: Relevant,
        })
    }

    /// Strip the target to the internal parts.
    ///
    /// # Safety
    ///
    /// Surface and swapchain must be destroyed immediately.
    pub unsafe fn dispose(self) -> (Window, vk::SurfaceKHR, vk::SwapchainKHR) {
        self.relevant.dispose();
        (self.window, self.surface, self.swapchain)
    }

    /// Get raw surface handle.
    ///
    /// # Safety
    ///
    /// Raw handle usage should not violate this type valid usage.
    pub unsafe fn surface(&self) -> vk::SurfaceKHR {
        self.surface
    }

    /// Get raw surface handle.
    ///
    /// # Safety
    ///
    /// Raw handle usage should not violate this type valid usage.
    pub unsafe fn swapchain(&self) -> vk::SwapchainKHR {
        self.swapchain
    }

    /// Get target current extent.
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    /// Get target current format.
    pub fn format(&self) -> vk::Format {
        self.format
    }

    /// Get raw handlers for the swapchain images.
    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    /// Acquire next image.
    pub fn next_image(&mut self, signal: vk::Semaphore) -> Result<NextImages<'_>, Error> {
        let index = unsafe {
            self.fp
                .acquire_next_image_khr(self.swapchain, !0, signal, vk::Fence::null())
                .map_err(Error::from)
        }?;

        Ok(NextImages {
            fp: &self.fp,
            swapchains: smallvec![self.swapchain],
            indices: smallvec![index],
        })
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct NextImages<'a> {
    #[derivative(Debug = "ignore")]
    fp: &'a Swapchain,
    swapchains: SmallVec<[vk::SwapchainKHR; 4]>,
    indices: SmallVec<[u32; 4]>,
}

impl<'a> NextImages<'a> {
    /// Get indices.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Present images by the queue.
    pub fn queue_present(self, queue: vk::Queue, wait: &[vk::Semaphore]) -> Result<(), Error> {
        assert_eq!(self.swapchains.len(), self.indices.len());
        unsafe {
            // TODO: ???
            let mut results = std::iter::repeat(ash::vk::Result::SUCCESS)
                .take(self.swapchains.len())
                .collect::<SmallVec<[_; 4]>>();
            self.fp
                .queue_present_khr(
                    queue,
                    &vk::PresentInfoKHR::builder()
                        .wait_semaphores(wait)
                        .swapchains(&self.swapchains)
                        .image_indices(&self.indices)
                        .results(&mut results)
                        .build(),
                ).map_err(Error::from)
        }
    }
}
