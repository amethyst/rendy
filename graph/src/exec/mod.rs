use std::marker::PhantomData;

use crate::core::hal;
use crate::resource::Image;
use crate::scheduler::{
    interface::{ImageToken, SemaphoreId},
};

pub struct ExecCtx<B: hal::Backend> {
    phantom: PhantomData<B>,
}

impl<B: hal::Backend> ExecCtx<B> {

    /// Return the given semaphore to the render graphs internal pool.
    ///
    /// The render graph will make sure to only reuse the semaphore after
    /// the currently executing graph has finished executing.
    pub fn return_semaphore(&mut self, semaphore: B::Semaphore) {
        todo!()
    }

    pub fn get_image(&self) -> &Image<B> {
        todo!()
    }

    /// Fetches a provided swapchain image back from the graph.
    ///
    /// This can only be called if:
    /// * `ImageId` is a provided image of the swapchain image type
    /// * The current entity is the last use of `ImageId`
    ///
    /// If any of these are not true, it will panic.
    pub fn fetch_swapchain_image(&mut self, image_token: ImageToken) -> <B::Surface as hal::window::PresentationSurface<B>>::SwapchainImage {
        todo!()
    }

    pub fn fetch_semaphore(&mut self, semaphore_id: SemaphoreId) -> B::Semaphore {
        todo!()
    }
}
