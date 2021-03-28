use crate::Node;
use crate::graph::{GraphConstructCtx, GraphImage};
use crate::parameter::{Parameter, ParameterStore};
use crate::factory::Factory;
use crate::scheduler::{
    ImageId,
    interface::{GraphCtx, EntityCtx},
    resources::{ImageMode, ImageInfo, ProvidedImageUsage, ImageUsage},
};
use crate::wsi::Surface;

use rendy_core::hal;
use hal::image::FramebufferAttachment;
use hal::window::{PresentMode, Extent2D, SwapchainConfig};
use hal::format::{Format, ChannelType};

use crate::GraphBorrowable;

pub struct Present<B: hal::Backend> {
    swapchain_config: SwapchainConfig,
    framebuffer_attachment: FramebufferAttachment,

    surface: GraphBorrowable<Surface<B>>,
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

    pub fn new(factory: &Factory<B>, mut surface: Surface<B>, extent: Extent2D) -> Self {
        let (swapchain_config, framebuffer_attachment) = make_swapchain_config(
            factory, &surface, extent);

        unsafe {
            surface.configure_swapchain(
                factory.device(),
                swapchain_config.clone(),
            ).unwrap();
        }

        Self {
            swapchain_config,
            framebuffer_attachment,

            surface: GraphBorrowable::new(surface),
        }
    }

}

impl<B: hal::Backend> Node<B> for Present<B> {
    type Argument = ImageId;
    type Result = ();

    fn construct(
        &mut self,
        factory: &Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        in_image_id: ImageId,
    ) -> Result<(), ()> {

        let sc_image = loop {
            let surface = self.surface.borrow_mut();

            match unsafe { surface.acquire_image(!0) } {
                Ok((sc_image, _suboptimal)) => break sc_image,
                // Swapchain is out of date, we need to recreate and retry.
                Err(hal::window::AcquireError::OutOfDate(_)) => (),
                err => {
                    err.unwrap();
                    unreachable!();
                },
            }

            let (swapchain_config, framebuffer_attachment) = make_swapchain_config(
                factory, surface, self.swapchain_config.extent);
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
                surface.unconfigure_swapchain(factory.device());

                surface.configure_swapchain(
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
                levels: 1,
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

        // Perform a present.
        ctx.present(self.surface.take_borrow(), target_image_id, |node, result| {
            println!("TODO handle present result {:?}", result);
        });

        Ok(())
    }
}
