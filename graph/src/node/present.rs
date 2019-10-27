//! Defines present node.

use crate::{
    command::{
        CommandBuffer, CommandPool, ExecutableState, Families, Family, FamilyId, Fence, MultiShot,
        PendingState, Queue, SimultaneousUse, Submission, Submit,
    },
    factory::Factory,
    frame::Frames,
    graph::GraphContext,
    node::{
        gfx_acquire_barriers, gfx_release_barriers, BufferAccess, DynNode, ImageAccess, NodeBuffer,
        NodeBuildError, NodeBuilder, NodeImage,
    },
    wsi::{Surface, Target},
    BufferId, ImageId, NodeId,
};

#[derive(Debug)]
struct ForImage<B: rendy_core::hal::Backend> {
    acquire: B::Semaphore,
    release: B::Semaphore,
    submit: Submit<B, SimultaneousUse>,
    buffer: CommandBuffer<
        B,
        rendy_core::hal::queue::QueueType,
        PendingState<ExecutableState<MultiShot<SimultaneousUse>>>,
    >,
}

impl<B: rendy_core::hal::Backend> ForImage<B> {
    unsafe fn dispose(
        self,
        factory: &Factory<B>,
        pool: &mut CommandPool<B, rendy_core::hal::queue::QueueType>,
    ) {
        drop(self.submit);
        factory.destroy_semaphore(self.acquire);
        factory.destroy_semaphore(self.release);
        pool.free_buffers(Some(self.buffer.mark_complete()));
    }
}

/// Node that presents images to the surface.
#[derive(Debug)]
pub struct PresentNode<B: rendy_core::hal::Backend> {
    per_image: Vec<ForImage<B>>,
    free_acquire: B::Semaphore,
    target: Target<B>,
    pool: CommandPool<B, rendy_core::hal::queue::QueueType>,
    input_image: NodeImage,
    blit_filter: rendy_core::hal::image::Filter,
}

// Raw pointer destroys Send/Sync autoimpl, but it's always from the same graph.
unsafe impl<B: rendy_core::hal::Backend> Sync for PresentNode<B> {}
unsafe impl<B: rendy_core::hal::Backend> Send for PresentNode<B> {}

impl<B> PresentNode<B>
where
    B: rendy_core::hal::Backend,
{
    /// Node builder.
    /// By default attempts to use 3 images in the swapchain with present mode priority:
    ///
    /// Mailbox > Fifo > Relaxed > Immediate.
    ///
    /// You can query the real image count and present mode which will be used with
    /// `PresentBuilder::image_count()` and `PresentBuilder::present_mode()`.
    pub fn builder(factory: &Factory<B>, surface: Surface<B>, image: ImageId) -> PresentBuilder<B> {
        use rendy_core::hal::window::PresentMode;

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

        PresentBuilder {
            surface,
            image,
            dependencies: Vec::new(),
            image_count,
            present_mode,
            caps,
            blit_filter: rendy_core::hal::image::Filter::Nearest,
        }
    }
}

fn create_per_image_data<B: rendy_core::hal::Backend>(
    ctx: &GraphContext<B>,
    input_image: &NodeImage,
    pool: &mut CommandPool<B, rendy_core::hal::queue::QueueType>,
    factory: &Factory<B>,
    target: &Target<B>,
    blit_filter: rendy_core::hal::image::Filter,
) -> Vec<ForImage<B>> {
    let input_image_res = ctx.get_image(input_image.id).expect("Image does not exist");

    let target_images = target.backbuffer();
    let buffers = pool.allocate_buffers(target_images.len());
    target_images
        .iter()
        .zip(buffers)
        .map(|(target_image, buf_initial)| {
            let mut buf_recording = buf_initial.begin(MultiShot(SimultaneousUse), ());
            let mut encoder = buf_recording.encoder();
            let (mut stages, mut barriers) =
                gfx_acquire_barriers(ctx, None, Some(input_image));
            stages.start |= rendy_core::hal::pso::PipelineStage::TRANSFER;
            stages.end |= rendy_core::hal::pso::PipelineStage::TRANSFER;
            barriers.push(rendy_core::hal::memory::Barrier::Image {
                states: (
                    rendy_core::hal::image::Access::empty(),
                    rendy_core::hal::image::Layout::Undefined,
                )
                    ..(
                        rendy_core::hal::image::Access::TRANSFER_WRITE,
                        rendy_core::hal::image::Layout::TransferDstOptimal,
                    ),
                families: None,
                target: target_image.raw(),
                range: rendy_core::hal::image::SubresourceRange {
                    aspects: rendy_core::hal::format::Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                },
            });
            log::trace!("Acquire {:?} : {:#?}", stages, barriers);
            unsafe {
                encoder.pipeline_barrier(
                    stages,
                    rendy_core::hal::memory::Dependencies::empty(),
                    barriers,
                );
            }

            let extents_differ = target_image.kind().extent() != input_image_res.kind().extent();
            let formats_differ = target_image.format() != input_image_res.format();

            if extents_differ || formats_differ
            {
                if formats_differ {
                    log::debug!("Present node is blitting because target format {:?} doesnt match image format {:?}", target_image.format(), input_image_res.format());
                }
                if extents_differ {
                    log::debug!("Present node is blitting because target extent {:?} doesnt match image extent {:?}", target_image.kind().extent(), input_image_res.kind().extent());
                }
                unsafe {
                    encoder.blit_image(
                        input_image_res.raw(),
                        input_image.layout,
                        target_image.raw(),
                        rendy_core::hal::image::Layout::TransferDstOptimal,
                        blit_filter,
                        Some(rendy_core::hal::command::ImageBlit {
                            src_subresource: rendy_core::hal::image::SubresourceLayers {
                                aspects: input_image.range.aspects,
                                level: 0,
                                layers: input_image.range.layers.start..input_image.range.layers.start + 1,
                            },
                            src_bounds: rendy_core::hal::image::Offset::ZERO
                                .into_bounds(&input_image_res.kind().extent()),
                            dst_subresource: rendy_core::hal::image::SubresourceLayers {
                                aspects: rendy_core::hal::format::Aspects::COLOR,
                                level: 0,
                                layers: 0..1,
                            },
                            dst_bounds: rendy_core::hal::image::Offset::ZERO
                                .into_bounds(&target_image.kind().extent()),
                        }),
                    );
                }
            } else {
                log::debug!("Present node is copying");
                unsafe {
                    encoder.copy_image(
                        input_image_res.raw(),
                        input_image.layout,
                        target_image.raw(),
                        rendy_core::hal::image::Layout::TransferDstOptimal,
                        Some(rendy_core::hal::command::ImageCopy {
                            src_subresource: rendy_core::hal::image::SubresourceLayers {
                                aspects: input_image.range.aspects,
                                level: 0,
                                layers: input_image.range.layers.start..input_image.range.layers.start + 1,
                            },
                            src_offset: rendy_core::hal::image::Offset::ZERO,
                            dst_subresource: rendy_core::hal::image::SubresourceLayers {
                                aspects: rendy_core::hal::format::Aspects::COLOR,
                                level: 0,
                                layers: 0..1,
                            },
                            dst_offset: rendy_core::hal::image::Offset::ZERO,
                            extent: rendy_core::hal::image::Extent {
                                width: target_image.kind().extent().width,
                                height: target_image.kind().extent().height,
                                depth: 1,
                            },
                        }),
                    );
                }
            }

            {
                let (mut stages, mut barriers) =
                    gfx_release_barriers(ctx, None, Some(input_image));
                stages.start |= rendy_core::hal::pso::PipelineStage::TRANSFER;
                stages.end |= rendy_core::hal::pso::PipelineStage::BOTTOM_OF_PIPE;
                barriers.push(rendy_core::hal::memory::Barrier::Image {
                    states: (
                        rendy_core::hal::image::Access::TRANSFER_WRITE,
                        rendy_core::hal::image::Layout::TransferDstOptimal,
                    )
                        ..(
                            rendy_core::hal::image::Access::empty(),
                            rendy_core::hal::image::Layout::Present,
                        ),
                    families: None,
                    target: target_image.raw(),
                    range: rendy_core::hal::image::SubresourceRange {
                        aspects: rendy_core::hal::format::Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                });

                log::trace!("Release {:?} : {:#?}", stages, barriers);
                unsafe {
                    encoder.pipeline_barrier(
                        stages,
                        rendy_core::hal::memory::Dependencies::empty(),
                        barriers,
                    );
                }
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

/// Presentation node description.
#[derive(Debug)]
pub struct PresentBuilder<B: rendy_core::hal::Backend> {
    surface: Surface<B>,
    image: ImageId,
    image_count: u32,
    present_mode: rendy_core::hal::window::PresentMode,
    caps: rendy_core::hal::window::SurfaceCapabilities,
    dependencies: Vec<NodeId>,
    blit_filter: rendy_core::hal::image::Filter,
}

impl<B> PresentBuilder<B>
where
    B: rendy_core::hal::Backend,
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
            .min(*self.caps.image_count.end())
            .max(*self.caps.image_count.start());
        self.image_count = image_count;
        self
    }

    /// Set up filter used for resizing when backbuffer size does not match source image size.
    ///
    /// Default is `Nearest`.
    pub fn with_blit_filter(mut self, filter: rendy_core::hal::image::Filter) -> Self {
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
    /// - present_modes_priority: A function which takes a `rendy_core::hal::PresentMode` and returns
    /// an `Option<usize>`. `None` indicates not to use this mode, and a higher number returned
    /// indicates a higher prioirity for that mode.
    ///
    /// ## Panics
    /// - Panics if none of the provided `PresentMode`s are supported.
    pub fn with_present_modes_priority<PF>(mut self, present_modes_priority: PF) -> Self
    where
        PF: Fn(rendy_core::hal::window::PresentMode) -> Option<usize>,
    {
        use rendy_core::hal::window::PresentMode;

        let priority_mode = [
            PresentMode::FIFO,
            PresentMode::MAILBOX,
            PresentMode::RELAXED,
            PresentMode::IMMEDIATE,
        ]
        .iter()
        .cloned()
        .filter(|&mode| self.caps.present_modes.contains(mode))
        .filter_map(|mode| present_modes_priority(mode).map(|p| (p, mode)))
        .max_by_key(|&(p, _)| p);

        if let Some((_, mode)) = priority_mode {
            self.present_mode = mode;
        } else {
            panic!(
                "No desired PresentModes are supported. Supported: {:#?}",
                self.caps.present_modes
            );
        }

        self
    }

    /// Get image count in presentable swapchain.
    pub fn image_count(&self) -> u32 {
        self.image_count
    }

    /// Get present mode used by node.
    pub fn present_mode(&self) -> rendy_core::hal::window::PresentMode {
        self.present_mode
    }
}

impl<B, T> NodeBuilder<B, T> for PresentBuilder<B>
where
    B: rendy_core::hal::Backend,
    T: ?Sized,
{
    fn family(&self, factory: &mut Factory<B>, families: &Families<B>) -> Option<FamilyId> {
        // Find correct queue family.
        families.find(|family| factory.surface_support(family.id(), &self.surface))
    }

    fn buffers(&self) -> Vec<(BufferId, BufferAccess)> {
        Vec::new()
    }

    fn images(&self) -> Vec<(ImageId, ImageAccess)> {
        vec![(
            self.image,
            ImageAccess {
                access: rendy_core::hal::image::Access::TRANSFER_READ,
                layout: rendy_core::hal::image::Layout::TransferSrcOptimal,
                usage: rendy_core::hal::image::Usage::TRANSFER_SRC,
                stages: rendy_core::hal::pso::PipelineStage::TRANSFER,
            },
        )]
    }

    fn dependencies(&self) -> Vec<NodeId> {
        self.dependencies.clone()
    }

    fn build<'a>(
        self: Box<Self>,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        _queue: usize,
        _aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Box<dyn DynNode<B, T>>, NodeBuildError> {
        assert_eq!(buffers.len(), 0);
        assert_eq!(images.len(), 1);

        let input_image = images.into_iter().next().unwrap();
        let extent = ctx
            .get_image(input_image.id)
            .expect("Context must contain node's image")
            .kind()
            .extent()
            .into();

        if !factory.surface_support(family.id(), &self.surface) {
            log::warn!(
                "Surface {:?} presentation is unsupported by family {:?} bound to the node",
                self.surface,
                family
            );
            return Err(NodeBuildError::QueueFamily(family.id()));
        }

        let target = factory
            .create_target(
                self.surface,
                extent,
                self.image_count,
                self.present_mode,
                rendy_core::hal::image::Usage::TRANSFER_DST,
            )
            .map_err(NodeBuildError::Swapchain)?;

        let mut pool = factory
            .create_command_pool(family)
            .map_err(NodeBuildError::OutOfMemory)?;

        let per_image = create_per_image_data(
            ctx,
            &input_image,
            &mut pool,
            factory,
            &target,
            self.blit_filter,
        );

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
    B: rendy_core::hal::Backend,
    T: ?Sized,
{
    unsafe fn run<'a>(
        &mut self,
        ctx: &GraphContext<B>,
        factory: &Factory<B>,
        queue: &mut Queue<B>,
        _aux: &T,
        _frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, rendy_core::hal::pso::PipelineStage)],
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
                                    rendy_core::hal::pso::PipelineStage::TRANSFER,
                                ))))
                                .signal(signals.iter().cloned().chain(Some(&for_image.release))),
                        ),
                        fence.take(),
                    );

                    match next.present(queue.raw(), Some(&for_image.release)) {
                        Ok(_) => break,
                        Err(e) => {
                            log::debug!(
                                "Swapchain present error after next_image is acquired: {:?}",
                                e
                            );
                            // recreate swapchain on next frame.
                            break;
                        }
                    }
                }
                Err(rendy_core::hal::window::AcquireError::OutOfDate) => {
                    // recreate swapchain and try again.
                }
                e => {
                    e.unwrap();
                    break;
                }
            }
            // Recreate swapchain when OutOfDate
            // The code has to execute after match due to mutable aliasing issues.

            // TODO: use retired swapchains once available in hal and remove that wait
            factory.wait_idle().unwrap();

            let extent = ctx
                .get_image(self.input_image.id)
                .expect("Context must contain node's image")
                .kind()
                .extent()
                .into();

            self.target
                .recreate(factory.physical(), factory.device(), extent)
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
            );
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
