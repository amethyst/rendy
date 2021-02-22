use crate::builder::{GraphConstructCtx, GraphImage};
use crate::factory::Factory;
use crate::scheduler::{
    ImageId,
    interface::{GraphCtx, EntityCtx},
    resources::{ImageMode, ImageInfo, ProvidedImageUsage, ImageUsage},
};
use crate::wsi::Surface;
use super::super::parameter::{Parameter, ParameterStore};
use super::super::builder::Node;

use rendy_core::hal;
use hal::image::FramebufferAttachment;
use hal::window::{PresentMode, Extent2D, SwapchainConfig};
use hal::format::{Format, ChannelType};

pub struct Present<B: hal::Backend> {
    image: Parameter<ImageId>,

    swapchain_config: SwapchainConfig,
    framebuffer_attachment: FramebufferAttachment,

    surface: Surface<B>,
}

pub fn make_swapchain_config<B: hal::Backend>(
    factory: &Factory<B>,
    surface: &Surface<B>,
    extent: Extent2D
) -> (SwapchainConfig, FramebufferAttachment) {
    let caps = factory.get_surface_capabilities(&surface);
    let formats = factory.get_surface_formats(&surface);

    //let image_count = 3
    //    .min(*caps.image_count.end())
    //    .max(*caps.image_count.start());

    //let present_mode = match () {
    //    _ if caps.present_modes.contains(PresentMode::FIFO) => PresentMode::FIFO,
    //    _ if caps.present_modes.contains(PresentMode::MAILBOX) => PresentMode::MAILBOX,
    //    _ if caps.present_modes.contains(PresentMode::RELAXED) => PresentMode::RELAXED,
    //    _ if caps.present_modes.contains(PresentMode::IMMEDIATE) => PresentMode::IMMEDIATE,
    //    _ => panic!("No known present modes found"),
    //};

    let format = formats.map_or(Format::Rgba8Srgb, |formats| {
        formats
            .iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .map(|format| *format)
            .unwrap_or(formats[0])
    });

    let swapchain_config = rendy_core::hal::window::SwapchainConfig::from_caps(&caps, format, extent);
    let framebuffer_attachment = swapchain_config.framebuffer_attachment();

    (swapchain_config, framebuffer_attachment)
}

impl<B: hal::Backend> Present<B> {

    pub fn new(factory: &Factory<B>, mut surface: Surface<B>, image: Parameter<ImageId>, extent: Extent2D) -> Self {
        let (swapchain_config, framebuffer_attachment) = make_swapchain_config(
            factory, &surface, extent);

        unsafe {
            surface.configure_swapchain(
                factory.device(),
                swapchain_config.clone(),
            ).unwrap();
        }

        Self {
            image,

            swapchain_config,
            framebuffer_attachment,

            surface,
        }
    }

}

impl<B: hal::Backend> Node<B> for Present<B> {
    type Result = ();

    fn construct(
        &mut self,
        factory: &mut Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        store: &ParameterStore,
    ) -> Result<(), ()> {
        let in_image_id = *store.get(self.image).unwrap();

        let sc_image = loop {
            match unsafe { self.surface.acquire_image(!0) } {
                Ok((sc_image, _suboptimal)) => break sc_image,
                // Swapchain is out of date, we need to recreate and retry.
                Err(hal::window::AcquireError::OutOfDate(_)) => (),
                err => {
                    err.unwrap();
                    unreachable!();
                },
            }

            let (swapchain_config, framebuffer_attachment) = make_swapchain_config(
                factory, &self.surface, self.swapchain_config.extent);
            self.swapchain_config = swapchain_config;
            self.framebuffer_attachment = framebuffer_attachment;

            // Is this still relevant? We might be able to remove the wait.
            {
                // Recreate swapchain when OutOfDate
                // The code has to execute after match due to mutable aliasing issues.

                // TODO: use retired swapchains once available in hal and remove that wait
                factory.wait_idle().unwrap();
            }

            unsafe {
                self.surface.unconfigure_swapchain(factory.device());

                self.surface.configure_swapchain(
                    factory.device(),
                    self.swapchain_config.clone(),
                ).unwrap();
            }
        };

        let target_image_id = ctx.provide_image(
            ImageInfo {
                kind: Some(hal::image::Kind::D2(
                    self.swapchain_config.extent.width,
                    self.swapchain_config.extent.height,
                    self.swapchain_config.image_layers,
                    1,
                )),
                levels: 0,
                format: self.framebuffer_attachment.format,
                mode: ImageMode::DontCare {
                    transient: false,
                },
            },
            GraphImage::SwapchainImage(sc_image),
            None,
            Some(ProvidedImageUsage {
                layout: hal::image::Layout::Undefined,
                last_access: hal::image::Access::empty(),
            }),
        );

        // This will make the graph perform a time-travelling image move.
        // Well, we are not actually doing time travel, but it's still pretty
        // cool. `in_image_id` is required to be a graph owned image, and the
        // render graph will make sure it is backed with the provided target
        // image. If the images are incompatible, this will panic.
        ctx.move_image(in_image_id, target_image_id);

        // Get a sync point that represents the state of the swapchain image
        // after we are done rendering to it.
        let sync_point = ctx.sync_point_get(target_image_id);
        // Tell the render graph to signal our release semaphore on the sync
        // point generated above. This semaphore is then used in the present
        // call.
        let semaphore_id = ctx.sync_point_to_semaphore(sync_point);
        // ctx.sync_point_signal_semaphore(&self.release[idx as usize]);

        // We call present within a standalone graph entity.
        let mut present = ctx.standalone();

        let image_token = present.use_image(target_image_id, ImageUsage {
            layout: hal::image::Layout::Present,
            stages: hal::pso::PipelineStage::BOTTOM_OF_PIPE,
            access: hal::image::Access::empty(),
        }).unwrap();

        present.commit(|node, _factory, exec_ctx, queue| {
            let node = node.downcast_mut::<Present<B>>().unwrap();

            let mut render_semaphore = exec_ctx.fetch_semaphore(semaphore_id);
            let image = exec_ctx.fetch_swapchain_image(image_token);

            unsafe {
                node.surface.present(
                    queue.raw(),
                    image,
                    Some(&mut render_semaphore),
                ).unwrap();
            }

            exec_ctx.return_semaphore(render_semaphore);
        });

        Ok(())
    }
}
