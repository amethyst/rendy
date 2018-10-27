
use std::cmp::{min, max};

use ash::{
    version::{
        EntryV1_0,
        InstanceV1_0,
        DeviceV1_0,
    },
    extensions::{
        Surface,
        Swapchain,
    },
    vk::{
        PhysicalDevice,
        SurfaceKHR,
        SwapchainKHR,
        SwapchainCreateInfoKHR,
    },
};

use failure::Error;
use relevant::Relevant;
use winit::Window;

use NativeSurface;

pub struct Target {
    window: Window,
    surface: SurfaceKHR,
    swapchain: SwapchainKHR,
    relevant: Relevant,
}

impl Target {
    pub fn new(
        window: Window,
        image_count: u32,
        physical: PhysicalDevice,
        native_surface: &NativeSurface,
        surface: &Surface,
        swapchain: &Swapchain,
    ) -> Result<Self, Error> {
        let surface_khr = native_surface.create_surface(&window)?;

        let present_modes = unsafe { surface.get_physical_device_surface_present_modes_khr(physical, surface_khr) }?;
        debug!("Present modes: {:#?}", present_modes);

        let formats = unsafe { surface.get_physical_device_surface_formats_khr(physical, surface_khr) }?;
        debug!("Formats: {:#?}", formats);

        let capabilities = unsafe { surface.get_physical_device_surface_capabilities_khr(physical, surface_khr) }?;
        debug!("Capabilities: {:#?}", capabilities);

        let swapchain_khr = unsafe {
            swapchain.create_swapchain_khr(
                &SwapchainCreateInfoKHR::builder()
                    .surface(surface_khr)
                    .min_image_count(max(min(image_count, capabilities.max_image_count), capabilities.min_image_count))
                    .image_format(formats[0].format)
                    .image_extent(capabilities.current_extent)
                    .image_array_layers(1)
                    .image_usage(capabilities.supported_usage_flags)
                    .present_mode(present_modes[0])
                    .build(),
                None,
            )
        }?;

        trace!("Target created");
        
        Ok(Target {
            window,
            surface: surface_khr,
            swapchain: swapchain_khr,
            relevant: Relevant,
        })
    }

    pub unsafe fn dispose(self) -> (Window, SurfaceKHR, SwapchainKHR) {
        self.relevant.dispose();
        (self.window, self.surface, self.swapchain)
    }
}
