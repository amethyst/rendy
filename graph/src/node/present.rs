use crate::new::builder::GraphConstructCtx;
use crate::factory::Factory;
use crate::scheduler::{
    ImageId,
    interface::{GraphCtx, EntityCtx},
    resources::{ImageMode, ImageInfo, ProvidedImageUsage, ImageUsage},
};
use crate::wsi::{Surface, Target};
use super::super::parameter::{Parameter, ParameterStore};
use super::super::builder::Node;

use rendy_core::hal::window::{PresentMode, Extent2D};
use rendy_core::hal;

pub struct Present<B: hal::Backend> {
    image: Parameter<ImageId>,
    image_count: u32,
    present_mode: PresentMode,
    extent: Extent2D,

    target: Option<Target<B>>,

    free_acquire: B::Semaphore,
    acquire: Vec<B::Semaphore>,
    release: Vec<B::Semaphore>,
}

impl<B: hal::Backend> Present<B> {

    pub fn new(factory: &Factory<B>, surface: Surface<B>, image: Parameter<ImageId>, extent: Extent2D) -> Self {
        let caps = factory.get_surface_capabilities(&surface);

        let image_count = 3
            .min(*caps.image_count.end())
            .max(*caps.image_count.start());

        let present_mode = match () {
            _ if caps.present_modes.contains(PresentMode::FIFO) => PresentMode::FIFO,
            _ if caps.present_modes.contains(PresentMode::MAILBOX) => PresentMode::MAILBOX,
            _ if caps.present_modes.contains(PresentMode::RELAXED) => PresentMode::RELAXED,
            _ if caps.present_modes.contains(PresentMode::IMMEDIATE) => PresentMode::IMMEDIATE,
            _ => panic!("No known present modes found"),
        };

        let target = factory
            .create_target(
                surface,
                extent,
                image_count,
                present_mode,
                rendy_core::hal::image::Usage::TRANSFER_DST,
            )
            .unwrap();

        Self {
            image,
            image_count,
            present_mode,
            extent,

            target: Some(target),

            free_acquire: factory.create_semaphore().unwrap(),
            acquire: (0..image_count).map(|_| factory.create_semaphore().unwrap()).collect(),
            release: (0..image_count).map(|_| factory.create_semaphore().unwrap()).collect(),
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

        let target = self.target.as_mut().unwrap();

        let img_kind = target.backbuffer()[0].kind();
        let img_levels = target.backbuffer()[0].levels();
        let img_format = target.backbuffer()[0].format();

        let next = loop {
            match unsafe { target.next_image(&self.free_acquire) } {
                Ok(next) => {
                    break next;


                    //break;
                },
                Err(rendy_core::hal::window::AcquireError::OutOfDate) => {
                    // recreate swapchain and try again.
                },
                e => {
                    e.unwrap();
                    unreachable!();
                },
            }

            // Recreate swapchain when OutOfDate
            // The code has to execute after match due to mutable aliasing issues.

            // TODO: use retired swapchains once available in hal and remove that wait
            factory.wait_idle().unwrap();

            unsafe {
                target
                    .recreate(factory.physical(), factory.device(), self.extent)
                    .expect("Failed recreating swapchain");
            }
        };

        let idx = next[0];
        core::mem::swap(&mut self.acquire[idx as usize], &mut self.free_acquire);

        let target_image_id = ctx.provide_image(
            ImageInfo {
                kind: Some(img_kind),
                levels: img_levels,
                format: img_format,
                mode: ImageMode::DontCare {
                    transient: false,
                },
            },
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
        // ctx.sync_point_signal_semaphore(&self.release[idx as usize]);

        // We call present within a standalone graph entity.
        let mut present = ctx.standalone();

        // This call is required to mark the image as a dependency of the graph
        // entity that does the presenting, but the returned token is never
        // actually used. This is because `present` is called with the swapchain
        // and the image index, not the image object itself.
        let _image_token = present.use_image(target_image_id, ImageUsage {
            layout: hal::image::Layout::Present,
            stages: hal::pso::PipelineStage::BOTTOM_OF_PIPE,
            access: hal::image::Access::empty(),
        });

        present.commit(|factory, exec_ctx| {
        });

        Ok(())
    }
}
