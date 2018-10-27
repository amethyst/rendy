
use ash::{
    vk::{
        SurfaceKHR,
        SwapchainKHR,
    },
};

use relevant::Relevant;
use winit::Window;

pub struct Target {
    window: Window,
    surface: SurfaceKHR,
    swapchain: SwapchainKHR,
    relevant: Relevant,
}

impl Target {
    pub unsafe fn new(window: Window, surface: SurfaceKHR, swapchain: SwapchainKHR) -> Self {
        Target {
            window,
            surface,
            swapchain,
            relevant: Relevant,
        }
    }

    pub unsafe fn dispose(self) -> (Window, SurfaceKHR, SwapchainKHR) {
        self.relevant.dispose();
        (self.window, self.surface, self.swapchain)
    }
}
