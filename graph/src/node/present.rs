//! Defines present node.

use std::borrow::Borrow;

use crate::{
    command::{
        CommandBuffer, CommandPool, ExecutableState, Families, Family, FamilyId, Fence, MultiShot,
        PendingState, Queue, SimultaneousUse, Submission, Submit, IndividualReset, NoSimultaneousUse,
    },
    factory::Factory,
    frame::{
        cirque::{CirqueRef, CommandCirque},
        Frames,
    },
    graph::GraphContext,
    node::{
        gfx_acquire_barriers, gfx_release_barriers, BufferAccess, DynNode, ImageAccess, NodeBuffer,
        NodeBuildError, NodeBuilder, NodeImage,
    },
    wsi::Surface,
    BufferId, ImageId, NodeId,
};

use rendy_core::hal;
use hal::format::{Format, ChannelType};
use hal::image::FramebufferAttachment;
use hal::window::{PresentMode, Extent2D, SwapchainConfig};

fn make_swapchain_config<B: hal::Backend>(factory: &Factory<B>, surface: &Surface<B>, extent: Extent2D) -> (SwapchainConfig, FramebufferAttachment) {
    let caps = factory.get_surface_capabilities(surface);
    let formats = factory.get_surface_formats(surface);

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

#[derive(Debug)]
struct ForImage<B: rendy_core::hal::Backend> {
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
        factory.destroy_semaphore(self.release);
        pool.free_buffers(Some(self.buffer.mark_complete()));
    }
}

/// Presentation node description.
#[derive(Debug)]
pub struct PresentBuilder<B: rendy_core::hal::Backend> {
    surface: Surface<B>,
    image: ImageId,
    image_count: u32,
    present_mode: PresentMode,
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
        PF: Fn(PresentMode) -> Option<usize>,
    {
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
    pub fn present_mode(&self) -> PresentMode {
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

        let caps = factory.get_surface_capabilities(&self.surface);
        let formats = factory.get_surface_formats(&self.surface);

        let input_image = images.into_iter().next().unwrap();

        let extent;
        let format;
        {
            let img = ctx
                .get_image(input_image.id)
                .expect("Context must contain node's image");

            extent = img.kind().extent().into();
            format = img.format();
        }

        if !factory.surface_support(family.id(), &self.surface) {
            log::warn!(
                "Surface {:?} presentation is unsupported by family {:?} bound to the node",
                self.surface,
                family
            );
            return Err(NodeBuildError::QueueFamily(family.id()));
        }

        let swapchain_config = rendy_core::hal::window::SwapchainConfig::from_caps(&caps, format, extent);
        let fat = swapchain_config.framebuffer_attachment();

        unsafe {
            self.surface.configure_swapchain(
                factory.device(),
                swapchain_config,
            ).map_err(NodeBuildError::Swapchain)?;
        }

        let mut pool = factory
            .create_command_pool(family)
            .map_err(NodeBuildError::OutOfMemory)?;

        let command_cirque = CommandCirque::new();

        Ok(Box::new(PresentNode {
            release_idx: 0,
            release: (0..self.image_count).map(|_| factory.create_semaphore().unwrap()).collect(),
            pool,
            command_cirque,
            surface: self.surface,
            //per_image,
            input_image,
            blit_filter: self.blit_filter,
            swapchain_config,
        }))
    }
}

/// Node that presents images to the surface.
#[derive(Debug)]
pub struct PresentNode<B: rendy_core::hal::Backend> {
    //per_image: Vec<ForImage<B>>,
    release_idx: usize,
    release: Vec<B::Semaphore>,
    surface: Surface<B>,

    pool: CommandPool<B, rendy_core::hal::queue::QueueType, IndividualReset>,
    command_cirque: CommandCirque<B, rendy_core::hal::queue::QueueType>,

    input_image: NodeImage,
    blit_filter: rendy_core::hal::image::Filter,

    swapchain_config: SwapchainConfig,
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
        frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, rendy_core::hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        mut fence: Option<&mut Fence<B>>,
    ) {
        let input_image_res = ctx.get_image(self.input_image.id).expect("Image does not exist");

        let swapchain_image = loop {
            match self.surface.acquire_image(!0) {
                Ok((swapchain_image, _suboptimal)) => {
                    break swapchain_image;
                },
                Err(hal::window::AcquireError::OutOfDate(_)) => {
                    // recreate swapchain and try again
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

            let extent;
            let format;
            {
                let img = ctx
                    .get_image(self.input_image.id)
                    .expect("Context must contain node's image");

                extent = img.kind().extent().into();
                format = img.format();
            }

            let extent = ctx
                .get_image(self.input_image.id)
                .expect("Context must contain node's image")
                .kind()
                .extent()
                .into();

            let (swapchain_config, _fat) = make_swapchain_config(factory, &self.surface, extent);

            unsafe {
                self.surface.configure_swapchain(
                    factory.device(),
                    swapchain_config,
                ).unwrap();
            }
        };

        self.release_idx += 1;
        if self.release_idx >= self.release.len() {
            self.release_idx = 0;
        }

        let submit = self.command_cirque.encode(frames, &mut self.pool, |mut cbuf| {
            let index = cbuf.index();

            cbuf.or_init(|cbuf| {
                let mut cbuf = cbuf.begin(MultiShot(NoSimultaneousUse), ());
                let mut encoder = cbuf.encoder();

                let (mut stages, mut barriers) =
                    gfx_acquire_barriers(ctx, None, Some(&self.input_image));
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
                    target: swapchain_image.borrow(),
                    range: rendy_core::hal::image::SubresourceRange {
                        aspects: rendy_core::hal::format::Aspects::COLOR,
                        level_start: 0,
                        level_count: Some(1),
                        layer_start: 0,
                        layer_count: Some(1),
                    },
                });
                log::trace!("Acquire {:?} : {:#?}", stages, barriers);
                unsafe {
                    encoder.pipeline_barrier(
                        stages,
                        rendy_core::hal::memory::Dependencies::empty(),
                        barriers.drain(..),
                    );
                }

                let extents_differ = self.swapchain_config.extent.to_extent() != input_image_res.kind().extent();
                let formats_differ = self.swapchain_config.format != input_image_res.format();

                if extents_differ || formats_differ
                {
                    if formats_differ {
                        log::debug!("Present node is blitting because target format {:?} doesnt match image format {:?}", self.swapchain_config.format, input_image_res.format());
                    }
                    if extents_differ {
                        log::debug!("Present node is blitting because target extent {:?} doesnt match image extent {:?}", self.swapchain_config.extent.to_extent(), input_image_res.kind().extent());
                    }
                    unsafe {
                        encoder.blit_image(
                            input_image_res.raw(),
                            self.input_image.layout,
                            swapchain_image.borrow().raw(),
                            rendy_core::hal::image::Layout::TransferDstOptimal,
                            self.blit_filter,
                            std::iter::once(rendy_core::hal::command::ImageBlit {
                                src_subresource: rendy_core::hal::image::SubresourceLayers {
                                    aspects: self.input_image.range.aspects,
                                    level: 0,
                                    layers: self.input_image.range.layer_start..self.input_image.range.layer_start + 1,
                                },
                                src_bounds: rendy_core::hal::image::Offset::ZERO
                                    .into_bounds(&input_image_res.kind().extent()),
                                dst_subresource: rendy_core::hal::image::SubresourceLayers {
                                    aspects: rendy_core::hal::format::Aspects::COLOR,
                                    level: 0,
                                    layers: 0..1,
                                },
                                dst_bounds: rendy_core::hal::image::Offset::ZERO
                                    .into_bounds(&self.swapchain_config.extent.to_extent()),
                            }),
                        );
                    }
                } else {
                    log::debug!("Present node is copying");
                    unsafe {
                        encoder.copy_image(
                            input_image_res.raw(),
                            self.input_image.layout,
                            swapchain_image.borrow().raw(),
                            rendy_core::hal::image::Layout::TransferDstOptimal,
                            std::iter::once(rendy_core::hal::command::ImageCopy {
                                src_subresource: rendy_core::hal::image::SubresourceLayers {
                                    aspects: self.input_image.range.aspects,
                                    level: 0,
                                    layers: self.input_image.range.layer_start..self.input_image.range.layer_start + 1,
                                },
                                src_offset: rendy_core::hal::image::Offset::ZERO,
                                dst_subresource: rendy_core::hal::image::SubresourceLayers {
                                    aspects: rendy_core::hal::format::Aspects::COLOR,
                                    level: 0,
                                    layers: 0..1,
                                },
                                dst_offset: rendy_core::hal::image::Offset::ZERO,
                                extent: self.swapchain_config.extent.to_extent(),
                            }),
                        );
                    }
                }

                {
                    let (mut stages, mut barriers) =
                        gfx_release_barriers(ctx, None, Some(&self.input_image));
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
                        target: swapchain_image.borrow(),
                        range: rendy_core::hal::image::SubresourceRange {
                            aspects: rendy_core::hal::format::Aspects::COLOR,
                            level_start: 0,
                            level_count: Some(1),
                            layer_start: 0,
                            layer_count: Some(1),
                        },
                    });

                    log::trace!("Release {:?} : {:#?}", stages, barriers);
                    unsafe {
                        encoder.pipeline_barrier(
                            stages,
                            rendy_core::hal::memory::Dependencies::empty(),
                            barriers.drain(..),
                        );
                    }
                }

                cbuf.finish()
            })
        });

        queue.submit(
            Some(
                Submission::new()
                    .submits(std::iter::once(submit))
                    .signal(signals.iter().cloned().chain(Some(&self.release[self.release_idx]))),
            ),
            fence.take(),
        );

        self.surface.present(queue.raw(), swapchain_image, Some(&mut self.release[self.release_idx]));
    }

    unsafe fn dispose(mut self: Box<Self>, factory: &mut Factory<B>, _aux: &T) {
        for semaphore in self.release {
            factory.destroy_semaphore(semaphore);
        }

        factory.destroy_command_pool(self.pool);
        factory.destroy_surface(self.surface);
    }
}
