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

pub fn make_swapchain_config<B: hal::Backend>(
    factory: &Factory<B>,
    surface: &Surface<B>,
    extent: Extent2D
) -> (SwapchainConfig, FramebufferAttachment) {
    let caps = factory.get_surface_capabilities(&surface);
    let formats = factory.get_surface_formats(&surface);

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

pub struct Present<B: hal::Backend> {
    swapchain_config: SwapchainConfig,
    framebuffer_attachment: FramebufferAttachment,

    do_recreate: GraphBorrowable<bool>,
    fallback_extent: Extent2D,

    surface: GraphBorrowable<Surface<B>>,
}

impl<B: hal::Backend> Present<B> {

    pub fn new(factory: &Factory<B>, mut surface: Surface<B>, fallback_extent: Extent2D) -> Self {
        let (swapchain_config, framebuffer_attachment) = make_swapchain_config(
            factory, &surface, fallback_extent);

        unsafe {
            surface.configure_swapchain(
                factory.device(),
                swapchain_config.clone(),
            ).unwrap();
        }

        Self {
            swapchain_config,
            framebuffer_attachment,

            do_recreate: GraphBorrowable::new(false),
            fallback_extent,

            surface: GraphBorrowable::new(surface),
        }
    }

    pub fn set_fallback_extent(&mut self, fallback_extent: Extent2D) {
        if self.fallback_extent != fallback_extent {
            *self.do_recreate.borrow_mut() = true;
        }
        self.fallback_extent = fallback_extent;
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

        let do_recreate = self.do_recreate.borrow_mut();

        let sc_image = loop {
            let surface = self.surface.borrow_mut();

            // We have a scheduled recreate, skip aquisition for one iteration.
            if !*do_recreate {
                match unsafe { surface.acquire_image(!0) } {
                    Ok((sc_image, suboptimal)) => {
                        // If suboptimal, we continue with this image for this
                        // present, but schedule a recreate on next present.
                        if suboptimal.is_some() {
                            *do_recreate = true;
                        }

                        break sc_image;
                    },
                    // Swapchain is out of date, we need to recreate and retry.
                    Err(hal::window::AcquireError::OutOfDate(_)) => (),
                    err => {
                        err.unwrap();
                        unreachable!();
                    },
                }
            }

            println!("SWAPCHAIN RECREATE");

            let suggested_extent = unsafe {
                surface.extent(factory.physical())
            };

            let extent = if let Some(extent) = suggested_extent {
                extent
            } else {
                println!("surface has no suggested extent, using fallback");
                self.fallback_extent
            };

            let (swapchain_config, framebuffer_attachment) = make_swapchain_config(
                factory, surface, extent);
            self.swapchain_config = swapchain_config;
            self.framebuffer_attachment = framebuffer_attachment;

            unsafe {
                surface.unconfigure_swapchain(factory.device());

                surface.configure_swapchain(
                    factory.device(),
                    self.swapchain_config.clone(),
                ).unwrap();
            }

            *do_recreate = false;
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
                mode: ImageMode::DontCare,
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

        let mut do_recreate = self.do_recreate.take_borrow();

        // Perform a present.
        ctx.present(self.surface.take_borrow(), target_image_id, move |_node, result| {
            match result {
                Ok(Some(_suboptimal)) => {
                    println!("suboptimal on present, marking for recreate");
                    // We want to recreate on next present.
                    *do_recreate = true;
                },
                Ok(None) => (),
                Err(err) => {
                    println!("TODO swapchain present error! {:?}", err);
                },
            }
        });

        Ok(())
    }
}
