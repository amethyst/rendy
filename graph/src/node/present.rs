//! Defines present node.

use crate::{
    command::{
        CommandBuffer, CommandPool, ExecutableState, Family, FamilyId, Fence, MultiShot,
        PendingState, Queue, SimultaneousUse, Submission, Submit,
    },
    factory::Factory,
    frame::Frames,
    graph::GraphContext,
    node::{
        gfx_acquire_barriers, gfx_release_barriers, BufferAccess, DynNode, ImageAccess, NodeBuffer,
        NodeBuilder, NodeImage,
    },
    wsi::{Backbuffer, Surface, Target},
    BufferId, ImageId, NodeId,
};

#[derive(Debug)]
struct ForImage<B: gfx_hal::Backend> {
    acquire: B::Semaphore,
    release: B::Semaphore,
    submit: Submit<B, SimultaneousUse>,
    buffer: CommandBuffer<
        B,
        gfx_hal::QueueType,
        PendingState<ExecutableState<MultiShot<SimultaneousUse>>>,
    >,
}

impl<B: gfx_hal::Backend> ForImage<B> {
    unsafe fn dispose(self, factory: &Factory<B>, pool: &mut CommandPool<B, gfx_hal::QueueType>) {
        drop(self.submit);
        factory.destroy_semaphore(self.acquire);
        factory.destroy_semaphore(self.release);
        pool.free_buffers(Some(self.buffer.mark_complete()));
    }
}

/// Node that presents images to the surface.
#[derive(Debug)]
pub struct PresentNode<B: gfx_hal::Backend> {
    per_image: Vec<ForImage<B>>,
    free_acquire: B::Semaphore,
    target: Target<B>,
    pool: CommandPool<B, gfx_hal::QueueType>,
    input_image: NodeImage,
    blit_filter: gfx_hal::image::Filter,
}

// Raw pointer destroys Send/Sync autoimpl, but it's always from the same graph.
unsafe impl<B: gfx_hal::Backend> Sync for PresentNode<B> {}
unsafe impl<B: gfx_hal::Backend> Send for PresentNode<B> {}

impl<B> PresentNode<B>
where
    B: gfx_hal::Backend,
{
    /// Node builder.
    /// By default attempts to use 3 images in the swapchain with present mode priority:
    ///
    /// Mailbox > Fifo > Relaxed > Immediate.
    ///
    /// You can query the real image count and present mode which will be used with
    /// `PresentBuilder::image_count()` and `PresentBuilder::present_mode()`.
    pub fn builder(factory: &Factory<B>, surface: Surface<B>, image: ImageId) -> PresentBuilder<B> {
        let (caps, _f, present_modes_caps, _a) = factory.get_surface_compatibility(&surface);

        let img_count_caps = caps.image_count;
        let image_count = 3.min(img_count_caps.end).max(img_count_caps.start);

        let present_mode = *present_modes_caps
            .iter()
            .max_by_key(|mode| match mode {
                gfx_hal::PresentMode::Mailbox => 3,
                gfx_hal::PresentMode::Fifo => 2,
                gfx_hal::PresentMode::Relaxed => 1,
                gfx_hal::PresentMode::Immediate => 0,
            })
            .unwrap();

        PresentBuilder {
            surface,
            image,
            dependencies: Vec::new(),
            image_count,
            img_count_caps,
            present_mode,
            present_modes_caps,
            blit_filter: gfx_hal::image::Filter::Nearest,
        }
    }
}

fn create_per_image_data<B: gfx_hal::Backend>(
    ctx: &GraphContext<B>,
    input_image: &NodeImage,
    pool: &mut CommandPool<B, gfx_hal::QueueType>,
    factory: &Factory<B>,
    target: &Target<B>,
    blit_filter: gfx_hal::image::Filter,
) -> Result<Vec<ForImage<B>>, failure::Error> {
    let input_image_res = ctx.get_image(input_image.id).expect("Image does not exist");
    let input_extend = input_image_res.kind().extent();

    let per_image = match target.backbuffer() {
        Backbuffer::Images(target_images) => {
            let buffers = pool.allocate_buffers(target_images.len());
            target_images
                .iter()
                .zip(buffers)
                .map(|(target_image, buf_initial)| {
                    let mut buf_recording = buf_initial.begin(MultiShot(SimultaneousUse), ());
                    let mut encoder = buf_recording.encoder();
                    let (mut stages, mut barriers) =
                        gfx_acquire_barriers(ctx, None, Some(input_image));
                    stages.start |= gfx_hal::pso::PipelineStage::TRANSFER;
                    stages.end |= gfx_hal::pso::PipelineStage::TRANSFER;
                    barriers.push(gfx_hal::memory::Barrier::Image {
                        states: (
                            gfx_hal::image::Access::empty(),
                            gfx_hal::image::Layout::Undefined,
                        )
                            ..(
                                gfx_hal::image::Access::TRANSFER_WRITE,
                                gfx_hal::image::Layout::TransferDstOptimal,
                            ),
                        families: None,
                        target: target_image.raw(),
                        range: gfx_hal::image::SubresourceRange {
                            aspects: gfx_hal::format::Aspects::COLOR,
                            levels: 0..1,
                            layers: 0..1,
                        },
                    });
                    log::info!("Acquire {:?} : {:#?}", stages, barriers);
                    encoder.pipeline_barrier(
                        stages,
                        gfx_hal::memory::Dependencies::empty(),
                        barriers,
                    );

                    if target_image.kind().extent() != input_extend {
                        encoder.blit_image(
                            input_image_res.raw(),
                            input_image.layout,
                            target_image.raw(),
                            gfx_hal::image::Layout::TransferDstOptimal,
                            blit_filter,
                            Some(gfx_hal::command::ImageBlit {
                                src_subresource: gfx_hal::image::SubresourceLayers {
                                    aspects: gfx_hal::format::Aspects::COLOR,
                                    level: 0,
                                    layers: 0..1,
                                },
                                src_bounds: gfx_hal::image::Offset::ZERO.into_bounds(&input_extend),
                                dst_subresource: gfx_hal::image::SubresourceLayers {
                                    aspects: gfx_hal::format::Aspects::COLOR,
                                    level: 0,
                                    layers: 0..1,
                                },
                                dst_bounds: gfx_hal::image::Offset::ZERO
                                    .into_bounds(&target_image.kind().extent()),
                            }),
                        );
                    } else {
                        encoder.copy_image(
                            input_image_res.raw(),
                            input_image.layout,
                            target_image.raw(),
                            gfx_hal::image::Layout::TransferDstOptimal,
                            Some(gfx_hal::command::ImageCopy {
                                src_subresource: gfx_hal::image::SubresourceLayers {
                                    aspects: gfx_hal::format::Aspects::COLOR,
                                    level: 0,
                                    layers: 0..1,
                                },
                                src_offset: gfx_hal::image::Offset::ZERO,
                                dst_subresource: gfx_hal::image::SubresourceLayers {
                                    aspects: gfx_hal::format::Aspects::COLOR,
                                    level: 0,
                                    layers: 0..1,
                                },
                                dst_offset: gfx_hal::image::Offset::ZERO,
                                extent: gfx_hal::image::Extent {
                                    width: target_image.kind().extent().width,
                                    height: target_image.kind().extent().height,
                                    depth: 1,
                                },
                            }),
                        );
                    }

                    {
                        let (mut stages, mut barriers) =
                            gfx_release_barriers(ctx, None, Some(input_image));
                        stages.start |= gfx_hal::pso::PipelineStage::TRANSFER;
                        stages.end |= gfx_hal::pso::PipelineStage::BOTTOM_OF_PIPE;
                        barriers.push(gfx_hal::memory::Barrier::Image {
                            states: (
                                gfx_hal::image::Access::TRANSFER_WRITE,
                                gfx_hal::image::Layout::TransferDstOptimal,
                            )
                                ..(
                                    gfx_hal::image::Access::empty(),
                                    gfx_hal::image::Layout::Present,
                                ),
                            families: None,
                            target: target_image.raw(),
                            range: gfx_hal::image::SubresourceRange {
                                aspects: gfx_hal::format::Aspects::COLOR,
                                levels: 0..1,
                                layers: 0..1,
                            },
                        });

                        log::info!("Release {:?} : {:#?}", stages, barriers);
                        encoder.pipeline_barrier(
                            stages,
                            gfx_hal::memory::Dependencies::empty(),
                            barriers,
                        );
                    }

                    let (submit, buffer) = buf_recording.finish().submit();

                    ForImage {
                        submit,
                        buffer,
                        acquire: factory.create_semaphore().unwrap(),
                        release: factory.create_semaphore().unwrap(),
                    }
                })
                .collect()
        }
        _ => unimplemented!(),
    };

    Ok(per_image)
}

/// Presentation node description.
#[derive(Debug)]
pub struct PresentBuilder<B: gfx_hal::Backend> {
    surface: Surface<B>,
    image: ImageId,
    image_count: u32,
    img_count_caps: std::ops::Range<u32>,
    present_modes_caps: Vec<gfx_hal::PresentMode>,
    present_mode: gfx_hal::PresentMode,
    dependencies: Vec<NodeId>,
    blit_filter: gfx_hal::image::Filter,
}

impl<B> PresentBuilder<B>
where
    B: gfx_hal::Backend,
{
    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn add_dependency(&mut self, dependency: NodeId) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: NodeId) -> Self {
        self.add_dependency(dependency);
        self
    }

    /// Request a number of images in the swapchain. This is not guaranteed
    /// to be the final image count, but it will be if supported by the hardware.
    ///
    /// Check `PresentBuilder::image_count()` after calling this function but before
    /// building to see the final image count.
    pub fn with_image_count(mut self, image_count: u32) -> Self {
        let image_count = image_count
            .min(self.img_count_caps.end)
            .max(self.img_count_caps.start);
        self.image_count = image_count;
        self
    }

    /// Set up filter used for resizing when backbuffer size does not match source image size.
    ///
    /// Default is `Nearest`.
    pub fn with_blit_filter(mut self, filter: gfx_hal::image::Filter) -> Self {
        self.blit_filter = filter;
        self
    }

    /// Request a priority of present modes when creating the swapchain for the
    /// PresentNode. Lower index means higher priority.
    ///
    /// Check `PresentBuilder::present_mode()` after calling this function but before
    /// building to see the final present mode.
    ///
    /// ## Parameters
    /// - present_modes_priority: A function which takes a `gfx_hal::PresentMode` and returns
    /// an `Option<usize>`. `None` indicates not to use this mode, and a higher number returned
    /// indicates a higher prioirity for that mode.
    ///
    /// ## Panics
    /// - Panics if none of the provided `PresentMode`s are supported.
    pub fn with_present_modes_priority<PF>(mut self, present_modes_priority: PF) -> Self
    where
        PF: Fn(gfx_hal::PresentMode) -> Option<usize>,
    {
        if !self
            .present_modes_caps
            .iter()
            .any(|m| present_modes_priority(*m).is_some())
        {
            panic!(
                "No desired PresentModes are supported. Supported: {:#?}",
                self.present_modes_caps
            );
        }
        self.present_mode = *self
            .present_modes_caps
            .iter()
            .max_by_key(|&mode| present_modes_priority(*mode))
            .unwrap();
        self
    }

    pub fn image_count(&self) -> u32 {
        self.image_count
    }

    pub fn present_mode(&self) -> gfx_hal::PresentMode {
        self.present_mode
    }
}

impl<B, T> NodeBuilder<B, T> for PresentBuilder<B>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    fn family(&self, factory: &mut Factory<B>, families: &[Family<B>]) -> Option<FamilyId> {
        // Find correct queue family.
        families
            .iter()
            .find(|family| factory.surface_support(family.id(), self.surface.raw()))
            .map(Family::id)
    }

    fn buffers(&self) -> Vec<(BufferId, BufferAccess)> {
        Vec::new()
    }

    fn images(&self) -> Vec<(ImageId, ImageAccess)> {
        vec![(
            self.image,
            ImageAccess {
                access: gfx_hal::image::Access::TRANSFER_READ,
                layout: gfx_hal::image::Layout::TransferSrcOptimal,
                usage: gfx_hal::image::Usage::TRANSFER_SRC,
                stages: gfx_hal::pso::PipelineStage::TRANSFER,
            },
        )]
    }

    fn dependencies(&self) -> Vec<NodeId> {
        self.dependencies.clone()
    }

    fn build<'a>(
        self: Box<Self>,
        ctx: &mut GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        _queue: usize,
        _aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Box<dyn DynNode<B, T>>, failure::Error> {
        assert_eq!(buffers.len(), 0);
        assert_eq!(images.len(), 1);

        let input_image = images.into_iter().next().unwrap();

        let target = factory.create_target(
            self.surface,
            self.image_count,
            self.present_mode,
            gfx_hal::image::Usage::TRANSFER_DST,
        )?;

        let mut pool = factory.create_command_pool(family)?;

        let per_image = create_per_image_data(
            ctx,
            &input_image,
            &mut pool,
            factory,
            &target,
            self.blit_filter,
        )?;

        Ok(Box::new(PresentNode {
            free_acquire: factory.create_semaphore().unwrap(),
            pool,
            target,
            per_image,
            input_image,
            blit_filter: self.blit_filter,
        }))
    }
}

impl<B, T> DynNode<B, T> for PresentNode<B>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    unsafe fn run<'a>(
        &mut self,
        ctx: &GraphContext<B>,
        factory: &Factory<B>,
        queue: &mut Queue<B>,
        _aux: &T,
        _frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        mut fence: Option<&mut Fence<B>>,
    ) {
        loop {
            match self.target.next_image(&self.free_acquire) {
                Ok(next) => {
                    log::trace!("Present: {:#?}", next);
                    let ref mut for_image = self.per_image[next[0] as usize];
                    core::mem::swap(&mut for_image.acquire, &mut self.free_acquire);

                    queue.submit(
                        Some(
                            Submission::new()
                                .submits(Some(&for_image.submit))
                                .wait(waits.iter().cloned().chain(Some((
                                    &for_image.acquire,
                                    gfx_hal::pso::PipelineStage::TRANSFER,
                                ))))
                                .signal(signals.iter().cloned().chain(Some(&for_image.release))),
                        ),
                        fence.take(),
                    );

                    match next.present(queue.raw(), Some(&for_image.release)) {
                        Err(e) => {
                            log::error!("Fix swapchain error: {}", e);
                            // recreate swapchain and try again.
                        }
                        _ => {
                            break;
                        }
                    }
                }
                Err(gfx_hal::window::AcquireError::OutOfDate) => {
                    // recreate swapchain and try again.
                }
                e => {
                    e.unwrap();
                    break;
                }
            }
            // Recreate swapchain when OutOfDate
            // The code has to execute after match due to mutable aliasing issues.
            factory.wait_idle().unwrap();
            self.target
                .recreate(factory.physical(), factory.device())
                .expect("Failed recreating swapchain");

            for data in self.per_image.drain(..) {
                data.dispose(factory, &mut self.pool);
            }

            self.per_image = create_per_image_data(
                ctx,
                &self.input_image,
                &mut self.pool,
                factory,
                &self.target,
                self.blit_filter,
            )
            .expect("Failed recreating swapchain data");
        }
    }

    unsafe fn dispose(mut self: Box<Self>, factory: &mut Factory<B>, _aux: &T) {
        for data in self.per_image {
            data.dispose(factory, &mut self.pool);
        }

        factory.destroy_semaphore(self.free_acquire);
        factory.destroy_command_pool(self.pool);
        factory.destroy_target(self.target);
    }
}
