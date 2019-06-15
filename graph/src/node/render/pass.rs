use {
    crate::{
        command::{
            CommandBuffer, CommandPool, ExecutableState, Family, FamilyId, Fence, Graphics,
            IndividualReset, MultiShot, NoSimultaneousUse, PendingState, Queue, QueueId,
            SecondaryLevel, SimultaneousUse, Submission, Submit, Supports,
        },
        factory::Factory,
        frame::{
            cirque::{CirqueRef, CommandCirque},
            Frames,
        },
        graph::GraphContext,
        node::{
            gfx_acquire_barriers, gfx_release_barriers, is_metal,
            render::group::{RenderGroup, RenderGroupBuilder},
            BufferAccess, DynNode, ImageAccess, NodeBuffer, NodeBuilder, NodeImage,
        },
        wsi::{Surface, Target},
        BufferId, ImageId, NodeId,
    },
    either::Either,
    gfx_hal::{image::Layout, Backend, Device as _},
    std::{cmp::min, collections::HashMap},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RenderPassSurface;

type Attachment = Either<ImageId, RenderPassSurface>;

/// Build for rendering sub-pass.
#[derive(derivative::Derivative)]
#[derivative(Default(bound = ""), Debug(bound = ""))]
pub struct SubpassBuilder<B: Backend, T: ?Sized> {
    groups: Vec<Box<dyn RenderGroupBuilder<B, T>>>,
    inputs: Vec<Attachment>,
    colors: Vec<Attachment>,
    depth_stencil: Option<Attachment>,
    dependencies: Vec<NodeId>,
}

impl<B, T> SubpassBuilder<B, T>
where
    B: Backend,
    T: ?Sized,
{
    /// Create new empty subpass builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add render group to this subpass.
    pub fn add_group<R>(&mut self, group: R) -> &mut Self
    where
        R: RenderGroupBuilder<B, T> + 'static,
    {
        self.groups.push(Box::new(group));
        self
    }

    /// Add render group to this subpass.
    pub fn with_group<R>(mut self, group: R) -> Self
    where
        R: RenderGroupBuilder<B, T> + 'static,
    {
        self.add_group(group);
        self
    }

    /// Add input attachment to the subpass.
    pub fn add_input(&mut self, input: ImageId) -> &mut Self {
        self.inputs.push(Either::Left(input));
        self
    }

    /// Add input attachment to the subpass.
    pub fn with_input(mut self, input: ImageId) -> Self {
        self.add_input(input);
        self
    }

    /// Add color attachment to the subpass.
    pub fn add_color(&mut self, color: ImageId) -> &mut Self {
        self.colors.push(Either::Left(color));
        self
    }

    /// Add color attachment to the subpass.
    pub fn with_color(mut self, color: ImageId) -> Self {
        self.add_color(color);
        self
    }

    /// Add surface as color attachment to the subpass.
    pub fn add_color_surface(&mut self) -> &mut Self {
        self.colors.push(Either::Right(RenderPassSurface));
        self
    }

    /// Add surface as color attachment to the subpass.
    pub fn with_color_surface(mut self) -> Self {
        self.add_color_surface();
        self
    }

    /// Set depth-stencil attachment to the subpass.
    pub fn set_depth_stencil(&mut self, depth_stencil: ImageId) -> &mut Self {
        self.depth_stencil = Some(Either::Left(depth_stencil));
        self
    }

    /// Set depth-stencil attachment to the subpass.
    pub fn with_depth_stencil(mut self, depth_stencil: ImageId) -> Self {
        self.set_depth_stencil(depth_stencil);
        self
    }

    /// Set surface as depth-stencil attachment to the subpass.
    pub fn set_depth_stencil_surface(&mut self) -> &mut Self {
        self.depth_stencil = Some(Either::Right(RenderPassSurface));
        self
    }

    /// Set surface as depth-stencil attachment to the subpass.
    pub fn with_depth_stencil_surface(mut self) -> Self {
        self.set_depth_stencil_surface();
        self
    }

    /// Add dependency.
    /// `RenderPassNode` will be placed after its dependencies.
    pub fn add_dependency(&mut self, dependency: NodeId) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add dependency.
    /// `RenderPassNode` will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: NodeId) -> Self {
        self.add_dependency(dependency);
        self
    }

    /// Make render pass from subpass.
    pub fn into_pass(self) -> RenderPassNodeBuilder<B, T> {
        RenderPassNodeBuilder::new().with_subpass(self)
    }
}

/// Builder for render-pass node.
#[derive(derivative::Derivative)]
#[derivative(Default(bound = ""), Debug(bound = ""))]
pub struct RenderPassNodeBuilder<B: Backend, T: ?Sized> {
    subpasses: Vec<SubpassBuilder<B, T>>,
    surface: Option<(Surface<B>, Option<gfx_hal::command::ClearValue>)>,
}

impl<B, T> RenderPassNodeBuilder<B, T>
where
    B: Backend,
    T: ?Sized,
{
    /// Make render pass node builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add sub-pass to the render-pass.
    pub fn add_subpass(&mut self, subpass: SubpassBuilder<B, T>) -> &mut Self {
        self.subpasses.push(subpass);
        self
    }

    /// Add sub-pass to the render-pass.
    pub fn with_subpass(mut self, subpass: SubpassBuilder<B, T>) -> Self {
        self.add_subpass(subpass);
        self
    }

    /// Add surface to the render pass.
    pub fn add_surface(
        &mut self,
        surface: Surface<B>,
        clear: Option<gfx_hal::command::ClearValue>,
    ) -> &mut Self {
        assert!(
            self.surface.is_none(),
            "Only one surface can be attachend to rende pass"
        );
        self.surface = Some((surface, clear));
        self
    }

    /// Add surface to the render pass.
    pub fn with_surface(
        mut self,
        surface: Surface<B>,
        clear: Option<gfx_hal::command::ClearValue>,
    ) -> Self {
        self.add_surface(surface, clear);
        self
    }
}

impl<B, T> NodeBuilder<B, T> for RenderPassNodeBuilder<B, T>
where
    B: Backend,
    T: ?Sized + 'static,
{
    fn family(&self, _factory: &mut Factory<B>, families: &[Family<B>]) -> Option<FamilyId> {
        families
            .iter()
            .find(|family| Supports::<Graphics>::supports(&family.capability()).is_some())
            .map(|family| family.id())
    }

    fn buffers(&self) -> Vec<(BufferId, BufferAccess)> {
        let empty = BufferAccess {
            access: gfx_hal::buffer::Access::empty(),
            usage: gfx_hal::buffer::Usage::empty(),
            stages: gfx_hal::pso::PipelineStage::empty(),
        };
        let mut buffers = HashMap::new();

        for subpass in &self.subpasses {
            for group in &subpass.groups {
                for (index, access) in group.buffers() {
                    let entry = buffers.entry(index).or_insert(empty);
                    entry.access |= access.access;
                    entry.usage |= access.usage;
                    entry.stages |= access.stages;
                }
            }
        }

        buffers.into_iter().collect()
    }

    fn images(&self) -> Vec<(ImageId, ImageAccess)> {
        let empty = ImageAccess {
            access: gfx_hal::image::Access::empty(),
            usage: gfx_hal::image::Usage::empty(),
            stages: gfx_hal::pso::PipelineStage::empty(),
            layout: Layout::Undefined,
        };
        let mut attachments = HashMap::new();
        let mut images = HashMap::new();

        for subpass in &self.subpasses {
            for &id in subpass.inputs.iter().filter_map(|e| e.as_ref().left()) {
                let entry = attachments.entry(id).or_insert(ImageAccess {
                    layout: Layout::ShaderReadOnlyOptimal,
                    ..empty
                });
                entry.access |= gfx_hal::image::Access::INPUT_ATTACHMENT_READ;
                entry.usage |= gfx_hal::image::Usage::INPUT_ATTACHMENT;
                entry.stages |= gfx_hal::pso::PipelineStage::FRAGMENT_SHADER;
            }

            for &id in subpass.colors.iter().filter_map(|e| e.as_ref().left()) {
                let entry = attachments.entry(id).or_insert(ImageAccess {
                    layout: Layout::ColorAttachmentOptimal,
                    ..empty
                });
                entry.access |= gfx_hal::image::Access::COLOR_ATTACHMENT_READ
                    | gfx_hal::image::Access::COLOR_ATTACHMENT_WRITE;
                entry.usage |= gfx_hal::image::Usage::COLOR_ATTACHMENT;
                entry.stages |= gfx_hal::pso::PipelineStage::COLOR_ATTACHMENT_OUTPUT;
            }

            if let Some(id) = subpass.depth_stencil.and_then(Either::left) {
                let entry = attachments.entry(id).or_insert(ImageAccess {
                    layout: Layout::DepthStencilAttachmentOptimal,
                    ..empty
                });
                entry.access |= gfx_hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ
                    | gfx_hal::image::Access::DEPTH_STENCIL_ATTACHMENT_WRITE;
                entry.usage |= gfx_hal::image::Usage::DEPTH_STENCIL_ATTACHMENT;
                entry.stages |= gfx_hal::pso::PipelineStage::EARLY_FRAGMENT_TESTS
                    | gfx_hal::pso::PipelineStage::LATE_FRAGMENT_TESTS;
            }

            for group in &subpass.groups {
                for (id, access) in group.images() {
                    assert!(
                        !attachments.contains_key(&id),
                        "Attachment image can't be used otherwise in render pass"
                    );
                    let entry = images.entry(id).or_insert(empty);
                    entry.access |= access.access;
                    entry.usage |= access.usage;
                    entry.stages |= access.stages;
                    entry.layout = common_layout(entry.layout, access.layout);
                }
            }
        }

        attachments.into_iter().chain(images.into_iter()).collect()
    }

    fn dependencies(&self) -> Vec<NodeId> {
        let mut dependencies: Vec<_> = self
            .subpasses
            .iter()
            .flat_map(|subpass| {
                subpass
                    .dependencies
                    .iter()
                    .cloned()
                    .chain(subpass.groups.iter().flat_map(|group| group.dependencies()))
            })
            .collect();
        dependencies.sort();
        dependencies.dedup();
        dependencies
    }

    fn build<'a>(
        self: Box<Self>,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        queue: usize,
        aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Box<dyn DynNode<B, T>>, failure::Error> {
        let mut surface_color_usage = false;
        let mut surface_depth_usage = false;

        let (mut surface, surface_clear) = self.surface.map_or((None, None), |(s, c)| (Some(s), c));
        log::debug!(
            "Build render pass node {} surface",
            surface.as_ref().map_or("without", |_| "with")
        );

        let mut attachments: Vec<Attachment> = self
            .subpasses
            .iter()
            .flat_map(|subpass| {
                subpass
                    .inputs
                    .iter()
                    .chain(subpass.colors.iter().inspect(|a| {
                        surface_color_usage = surface_color_usage || a.is_right();
                    }))
                    .chain(subpass.depth_stencil.as_ref().into_iter().inspect(|a| {
                        surface_depth_usage = surface_depth_usage || a.is_right();
                    }))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut surface_usage = gfx_hal::image::Usage::empty();
        if surface_color_usage {
            surface_usage |= gfx_hal::image::Usage::COLOR_ATTACHMENT;
        }
        if surface_depth_usage {
            surface_usage |= gfx_hal::image::Usage::DEPTH_STENCIL_ATTACHMENT;
        }

        if surface.is_some() {
            log::debug!("Surface usage {:#?}", surface_usage);
        } else {
            debug_assert_eq!(surface_usage, gfx_hal::image::Usage::empty());
        }

        attachments.sort();
        attachments.dedup();

        let find_attachment_node_image = |id: ImageId| -> &NodeImage {
            images
                .iter()
                .find(|a| a.id == id)
                .expect("Attachment image wasn't provided")
        };

        let mut framebuffer_width = u32::max_value();
        let mut framebuffer_height = u32::max_value();
        let mut framebuffer_layers = u16::max_value();

        let mut node_target = None;

        log::trace!("Configure attachments");

        let views: Vec<_> = attachments
            .iter()
            .map(|&attachment| -> Result<Vec<_>, failure::Error> {
                match attachment {
                    Either::Left(image_id) => {
                        log::debug!("Image {:?} attachment", image_id);

                        let node_image = find_attachment_node_image(image_id);
                        let image = ctx.get_image(image_id).expect("Image does not exist");
                        let extent = image.kind().extent();
                        framebuffer_width = min(framebuffer_width, extent.width);
                        framebuffer_height = min(framebuffer_height, extent.height);
                        framebuffer_layers = min(
                            framebuffer_layers,
                            node_image.range.layers.end - node_image.range.layers.start,
                        );
                        Ok(vec![unsafe {factory
                            .device()
                            .create_image_view(
                                image.raw(),
                                gfx_hal::image::ViewKind::D2,
                                image.format(),
                                gfx_hal::format::Swizzle::NO,
                                node_image.range.clone(),
                            )}?])
                    },
                    Either::Right(RenderPassSurface) => {
                        log::trace!("Surface attachment");

                        let surface = surface.take().expect("Render pass should be configured with Surface instance if at least one subpass uses surface attachment");
                        let surface_extent = unsafe {
                            surface.extent(factory.physical()).unwrap_or(gfx_hal::window::Extent2D { width: framebuffer_width, height: framebuffer_height })
                        };

                        log::debug!("Surface extent {:#?}", surface_extent);

                        let target = factory.create_target(
                            surface,
                            surface_extent,
                            3,
                            gfx_hal::window::PresentMode::Fifo,
                            surface_usage,
                        )?;

                        framebuffer_width = min(framebuffer_width, target.extent().width);
                        framebuffer_height = min(framebuffer_height, target.extent().height);
                        framebuffer_layers = min(
                            framebuffer_layers,
                            target.backbuffer()[0].layers(),
                        );

                        let views = target.backbuffer().iter().map(|image| unsafe {
                            factory
                                .device()
                                .create_image_view(
                                    image.raw(),
                                    gfx_hal::image::ViewKind::D2,
                                    image.format(),
                                    gfx_hal::format::Swizzle::NO,
                                    gfx_hal::image::SubresourceRange {
                                        aspects: image.format().surface_desc().aspects,
                                        levels: 0 .. 1,
                                        layers: 0 .. 1,
                                    },
                                ).map_err(failure::Error::from)
                        }).collect::<Result<Vec<_>, failure::Error>>()?;

                        node_target = Some(target);
                        Ok(views)
                    }
                }
            }).collect::<Result<Vec<_>, _>>()?
            .into_iter().flatten().collect();

        log::trace!("Configure render pass instance");

        let render_pass: B::RenderPass = {
            let pass_attachments: Vec<_> = attachments
                .iter()
                .map(|&attachment| {
                    let (format, clear, layout) = match attachment {
                        Either::Left(image_id) => {
                            let node_image = find_attachment_node_image(image_id);
                            let image = ctx.get_image(image_id).expect("Image does not exist");
                            (image.format(), node_image.clear, node_image.layout)
                        }
                        Either::Right(RenderPassSurface) => (
                            node_target
                                .as_ref()
                                .expect("Expect target created")
                                .backbuffer()[0]
                                .format(),
                            surface_clear,
                            gfx_hal::image::Layout::Present,
                        ),
                    };

                    gfx_hal::pass::Attachment {
                        format: Some(format),
                        ops: gfx_hal::pass::AttachmentOps {
                            load: if clear.is_some() {
                                gfx_hal::pass::AttachmentLoadOp::Clear
                            } else {
                                gfx_hal::pass::AttachmentLoadOp::Load
                            },
                            store: gfx_hal::pass::AttachmentStoreOp::Store,
                        },
                        stencil_ops: gfx_hal::pass::AttachmentOps::DONT_CARE,
                        layouts: if clear.is_some() {
                            gfx_hal::image::Layout::Undefined..layout
                        } else {
                            layout..layout
                        },
                        samples: 1,
                    }
                })
                .collect();

            log::debug!("Attachments {:#?}", pass_attachments);

            #[derive(Debug)]
            struct OwningSubpassDesc {
                inputs: Vec<(usize, Layout)>,
                colors: Vec<(usize, Layout)>,
                depth_stencil: Option<(usize, Layout)>,
            }

            let subpasses: Vec<_> = self
                .subpasses
                .iter()
                .map(|subpass| OwningSubpassDesc {
                    inputs: subpass
                        .inputs
                        .iter()
                        .map(|&i| {
                            (
                                attachments.iter().position(|&a| a == i).unwrap(),
                                match i {
                                    Either::Left(image_id) => {
                                        find_attachment_node_image(image_id).layout
                                    }
                                    Either::Right(RenderPassSurface) => {
                                        gfx_hal::image::Layout::ShaderReadOnlyOptimal
                                    }
                                },
                            )
                        })
                        .collect(),
                    colors: subpass
                        .colors
                        .iter()
                        .map(|&c| {
                            (
                                attachments.iter().position(|&a| a == c).unwrap(),
                                match c {
                                    Either::Left(image_id) => {
                                        find_attachment_node_image(image_id).layout
                                    }
                                    Either::Right(RenderPassSurface) => {
                                        gfx_hal::image::Layout::ColorAttachmentOptimal
                                    }
                                },
                            )
                        })
                        .collect(),
                    depth_stencil: subpass.depth_stencil.map(|ds| {
                        (
                            attachments.iter().position(|&a| a == ds).unwrap(),
                            match ds {
                                Either::Left(image_id) => {
                                    find_attachment_node_image(image_id).layout
                                }
                                Either::Right(RenderPassSurface) => {
                                    gfx_hal::image::Layout::DepthStencilAttachmentOptimal
                                }
                            },
                        )
                    }),
                })
                .collect();

            log::debug!("Subpasses {:#?}", subpasses);

            let subpasses: Vec<_> = subpasses
                .iter()
                .map(|subpass| gfx_hal::pass::SubpassDesc {
                    inputs: &subpass.inputs[..],
                    colors: &subpass.colors[..],
                    depth_stencil: subpass.depth_stencil.as_ref(),
                    resolves: &[],
                    preserves: &[],
                })
                .collect();

            let result = unsafe {
                factory
                    .device()
                    .create_render_pass(pass_attachments, subpasses, {
                        assert_eq!(
                            self.subpasses.len(),
                            1,
                            "TODO: Implement subpass dependencies to allow more than one subpass"
                        );
                        std::iter::empty::<gfx_hal::pass::SubpassDependency>()
                    })
            }
            .unwrap();

            log::trace!("RenderPass instance created");
            result
        };

        log::trace!(
            "Create {} framebuffers",
            views.len() - attachments.len() + 1
        );

        // Swapchain image views, if any, are last ones.
        let mut framebuffers = (attachments.len() - 1..views.len())
            .map(|i| unsafe {
                log::trace!(
                    "Create framebuffer for views {}..{} and {}",
                    0,
                    attachments.len() - 1,
                    i,
                );
                factory.device().create_framebuffer(
                    &render_pass,
                    views[..attachments.len() - 1].iter().chain(Some(&views[i])),
                    gfx_hal::image::Extent {
                        width: framebuffer_width,
                        height: framebuffer_height,
                        depth: framebuffer_layers as u32, // This is gfx-hal BUG as this parameter actually means framebuffer layers number,
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        log::trace!("Collect clears for render pass");

        let clears: Vec<_> = attachments
            .iter()
            .filter_map(|&a| match a {
                Either::Left(image_id) => find_attachment_node_image(image_id).clear,
                Either::Right(RenderPassSurface) => surface_clear,
            })
            .map(Into::into)
            .collect();

        let mut command_pool = factory
            .create_command_pool(family)?
            .with_capability()
            .expect("Graph must specify family that supports `Graphics`");

        let command_cirque = CommandCirque::new();

        let acquire = if !is_metal::<B>() {
            let (stages, barriers) = gfx_acquire_barriers(ctx, &buffers, &images);

            if !barriers.is_empty() {
                let initial = command_pool.allocate_buffers(1).pop().unwrap();
                let mut recording = initial.begin(MultiShot(SimultaneousUse), ());
                log::debug!("Acquire {:?} : {:#?}", stages, barriers);
                unsafe {
                    recording.encoder().pipeline_barrier(
                        stages,
                        gfx_hal::memory::Dependencies::empty(),
                        barriers,
                    );
                }
                let (acquire_submit, acquire_buffer) = recording.finish().submit();
                Some(BarriersCommands {
                    buffer: acquire_buffer,
                    submit: acquire_submit,
                })
            } else {
                None
            }
        } else {
            None
        };

        let release = if !is_metal::<B>() {
            let (stages, barriers) = gfx_release_barriers(ctx, &buffers, &images);

            if !barriers.is_empty() {
                let initial = command_pool.allocate_buffers(1).pop().unwrap();
                let mut recording = initial.begin(MultiShot(SimultaneousUse), ());
                log::debug!("Release {:?} : {:#?}", stages, barriers);
                unsafe {
                    recording.encoder().pipeline_barrier(
                        stages,
                        gfx_hal::memory::Dependencies::empty(),
                        barriers,
                    );
                }
                let (release_submit, release_buffer) = recording.finish().submit();
                Some(BarriersCommands {
                    buffer: release_buffer,
                    submit: release_submit,
                })
            } else {
                None
            }
        } else {
            None
        };

        let subpasses = self
            .subpasses
            .into_iter()
            .enumerate()
            .map(|(index, subpass)| {
                let subpass_colors = subpass.colors.len();
                let subpass_depth = subpass.depth_stencil.is_some();

                subpass
                    .groups
                    .into_iter()
                    .map(|group| {
                        assert_eq!(group.colors(), subpass_colors);
                        assert_eq!(group.depth(), subpass_depth);

                        let mut buffers = buffers.iter();
                        let mut images = images.iter();

                        let buffers: Vec<_> = group
                            .buffers()
                            .into_iter()
                            .map(|(id, _)| {
                                buffers
                                    .find(|b| b.id == id)
                                    .expect("Transient buffer wasn't provided")
                                    .clone()
                            })
                            .collect();
                        let images: Vec<_> = group
                            .images()
                            .into_iter()
                            .map(|(id, _)| {
                                images
                                    .find(|i| i.id == id)
                                    .expect("Transient image wasn't provided")
                                    .clone()
                            })
                            .collect();

                        group.build(
                            ctx,
                            factory,
                            QueueId {
                                family: family.id(),
                                index: queue,
                            },
                            aux,
                            framebuffer_width,
                            framebuffer_height,
                            gfx_hal::pass::Subpass {
                                index,
                                main_pass: &render_pass,
                            },
                            buffers,
                            images,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(|groups| SubpassNode { groups })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let node: Box<dyn DynNode<B, T>> = match node_target {
            Some(target) => {
                log::debug!("Construct RenderPassNodeWithSurface");
                Box::new(RenderPassNodeWithSurface {
                    common: RenderPassNodeCommon {
                        subpasses,

                        framebuffer_width,
                        framebuffer_height,
                        _framebuffer_layers: framebuffer_layers,

                        render_pass,
                        views,
                        clears,

                        command_pool,
                        command_cirque,

                        acquire,
                        release,

                        relevant: relevant::Relevant,
                    },

                    per_image: framebuffers
                        .into_iter()
                        .map(|fb| PerImage {
                            framebuffer: fb,
                            acquire: factory.create_semaphore().unwrap(),
                            release: factory.create_semaphore().unwrap(),
                            index: 0,
                        })
                        .collect(),
                    free_acquire: factory.create_semaphore().unwrap(),
                    target,
                })
            }
            None => {
                log::debug!("Construct RenderPassNodeWithoutSurface");
                Box::new(RenderPassNodeWithoutSurface {
                    common: RenderPassNodeCommon {
                        subpasses,

                        framebuffer_width,
                        framebuffer_height,
                        _framebuffer_layers: framebuffer_layers,

                        render_pass,
                        views,
                        clears,

                        command_pool,
                        command_cirque,

                        acquire,
                        release,

                        relevant: relevant::Relevant,
                    },
                    framebuffer: {
                        assert_eq!(framebuffers.len(), 1);
                        framebuffers.remove(0)
                    },
                })
            }
        };

        Ok(node)
    }
}

/// Subpass of the `RenderPassNode`.
#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
struct SubpassNode<B: Backend, T: ?Sized> {
    /// RenderGroups of pipelines to exeucte withing subpass.
    groups: Vec<Box<dyn RenderGroup<B, T>>>,
}

#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
struct BarriersCommands<B: Backend> {
    submit: Submit<B, SimultaneousUse, SecondaryLevel>,
    buffer: CommandBuffer<
        B,
        Graphics,
        PendingState<ExecutableState<MultiShot<SimultaneousUse>>>,
        SecondaryLevel,
        IndividualReset,
    >,
}

#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
struct RenderPassNodeCommon<B: Backend, T: ?Sized> {
    subpasses: Vec<SubpassNode<B, T>>,

    framebuffer_width: u32,
    framebuffer_height: u32,
    _framebuffer_layers: u16,

    render_pass: B::RenderPass,
    views: Vec<B::ImageView>,
    clears: Vec<gfx_hal::command::ClearValueRaw>,

    command_pool: CommandPool<B, Graphics, IndividualReset>,
    command_cirque: CommandCirque<B, Graphics>,

    acquire: Option<BarriersCommands<B>>,
    release: Option<BarriersCommands<B>>,

    relevant: relevant::Relevant,
}

impl<B, T> RenderPassNodeCommon<B, T>
where
    B: Backend,
    T: ?Sized,
{
    unsafe fn dispose(mut self, factory: &mut Factory<B>, aux: &T) {
        self.relevant.dispose();
        for subpass in self.subpasses {
            for group in subpass.groups {
                group.dispose(factory, aux)
            }
        }
        let pool = &mut self.command_pool;
        self.command_cirque.dispose(|buffer| {
            buffer.either_with(
                &mut *pool,
                |pool, executable| pool.free_buffers(Some(executable)),
                |pool, pending| {
                    let executable = pending.mark_complete();
                    pool.free_buffers(Some(executable))
                },
            );
        });
        if let Some(BarriersCommands { submit, buffer }) = self.acquire.take() {
            drop(submit);
            let executable = buffer.mark_complete();
            pool.free_buffers(Some(executable));
        }
        if let Some(BarriersCommands { submit, buffer }) = self.release.take() {
            drop(submit);
            let executable = buffer.mark_complete();
            pool.free_buffers(Some(executable));
        }
        factory.destroy_command_pool(self.command_pool.with_queue_type());

        for view in self.views {
            factory.device().destroy_image_view(view);
        }
        factory.device().destroy_render_pass(self.render_pass);
    }
}

#[derive(Debug)]
struct PerImage<B: Backend> {
    framebuffer: B::Framebuffer,
    acquire: B::Semaphore,
    release: B::Semaphore,
    index: usize,
}

#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
struct RenderPassNodeWithSurface<B: Backend, T: ?Sized> {
    common: RenderPassNodeCommon<B, T>,
    per_image: Vec<PerImage<B>>,
    free_acquire: B::Semaphore,
    target: Target<B>,
}

impl<B, T> DynNode<B, T> for RenderPassNodeWithSurface<B, T>
where
    B: Backend,
    T: ?Sized,
{
    unsafe fn run<'a>(
        &mut self,
        _ctx: &GraphContext<B>,
        factory: &Factory<B>,
        queue: &mut Queue<B>,
        aux: &T,
        frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&mut Fence<B>>,
    ) {
        let RenderPassNodeWithSurface {
            common:
                RenderPassNodeCommon {
                    subpasses,

                    framebuffer_width,
                    framebuffer_height,

                    render_pass,
                    clears,

                    command_cirque,
                    command_pool,

                    acquire,
                    release,
                    ..
                },
            target,
            free_acquire,
            per_image,
        } = self;

        let next = match target.next_image(&free_acquire) {
            Ok(next) => {
                log::trace!("Presentable image acquired: {:#?}", next);
                std::mem::swap(&mut per_image[next[0] as usize].acquire, free_acquire);
                Some(next)
            }
            Err(err) => {
                log::debug!("Swapchain acquisition error: {:#?}", err);
                None
            }
        };

        let submit = command_cirque.encode(frames, command_pool, |mut cbuf| {
            let index = cbuf.index();

            if let Some(next) = &next {
                let ref mut for_image = per_image[next[0] as usize];

                let force_record = subpasses.iter_mut().enumerate().fold(
                    false,
                    |force_record, (subpass_index, subpass)| {
                        subpass
                            .groups
                            .iter_mut()
                            .fold(force_record, |force_record, group| {
                                group
                                    .prepare(
                                        factory,
                                        queue.id(),
                                        index,
                                        gfx_hal::pass::Subpass {
                                            index: subpass_index,
                                            main_pass: &render_pass,
                                        },
                                        aux,
                                    )
                                    .force_record()
                                    || force_record
                            })
                    },
                );

                if force_record || for_image.index != index {
                    for_image.index = index;
                    cbuf = CirqueRef::Initial(cbuf.or_reset(|cbuf| cbuf.reset()));
                }
            }

            cbuf.or_init(|cbuf| {
                let mut cbuf = cbuf.begin(MultiShot(NoSimultaneousUse), ());
                let mut encoder = cbuf.encoder();

                if let Some(barriers) = &acquire {
                    encoder.execute_commands(std::iter::once(&barriers.submit));
                }

                if let Some(next) = &next {
                    let ref mut for_image = per_image[next[0] as usize];

                    let area = gfx_hal::pso::Rect {
                        x: 0,
                        y: 0,
                        w: *framebuffer_width as _,
                        h: *framebuffer_height as _,
                    };

                    let mut pass_encoder = encoder.begin_render_pass_inline(
                        &render_pass,
                        &for_image.framebuffer,
                        area,
                        &clears,
                    );

                    subpasses
                        .iter_mut()
                        .enumerate()
                        .for_each(|(subpass_index, subpass)| {
                            subpass.groups.iter_mut().for_each(|group| {
                                group.draw_inline(
                                    pass_encoder.reborrow(),
                                    index,
                                    gfx_hal::pass::Subpass {
                                        index: subpass_index,
                                        main_pass: &render_pass,
                                    },
                                    aux,
                                )
                            })
                        });

                    drop(pass_encoder);
                }

                if let Some(barriers) = &release {
                    encoder.execute_commands(std::iter::once(&barriers.submit));
                }
                cbuf.finish()
            })
        });

        log::trace!("Submit render pass");

        queue.submit(
            Some(
                Submission::new()
                    .submits(Some(submit))
                    .wait(waits.iter().cloned().chain(next.as_ref().map(|n| {
                        (
                            &per_image[n[0] as usize].acquire,
                            gfx_hal::pso::PipelineStage::TOP_OF_PIPE,
                        )
                    })))
                    .signal(
                        signals
                            .iter()
                            .cloned()
                            .chain(next.as_ref().map(|n| (&per_image[n[0] as usize].release))),
                    ),
            ),
            fence,
        );

        if let Some(next) = next {
            log::trace!("Present");
            let ref mut for_image = per_image[next[0] as usize];
            if let Err(err) = next.present(queue.raw(), Some(&for_image.release)) {
                log::debug!("Swapchain presentation error: {:#?}", err);
            }
        }
    }

    unsafe fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &T) {
        for per_image in self.per_image {
            factory.device().destroy_framebuffer(per_image.framebuffer);
            factory.destroy_semaphore(per_image.acquire);
            factory.destroy_semaphore(per_image.release);
        }
        self.common.dispose(factory, aux);
        factory.destroy_surface(factory.destroy_target(self.target));
    }
}

#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
struct RenderPassNodeWithoutSurface<B: Backend, T: ?Sized> {
    common: RenderPassNodeCommon<B, T>,
    framebuffer: B::Framebuffer,
}

impl<B, T> DynNode<B, T> for RenderPassNodeWithoutSurface<B, T>
where
    B: Backend,
    T: ?Sized,
{
    unsafe fn run<'a>(
        &mut self,
        _ctx: &GraphContext<B>,
        factory: &Factory<B>,
        queue: &mut Queue<B>,
        aux: &T,
        frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&mut Fence<B>>,
    ) {
        let RenderPassNodeWithoutSurface {
            common:
                RenderPassNodeCommon {
                    subpasses,

                    framebuffer_width,
                    framebuffer_height,

                    render_pass,
                    clears,

                    command_cirque,
                    command_pool,

                    acquire,
                    release,
                    ..
                },
            framebuffer,
        } = self;

        let submit = command_cirque.encode(frames, command_pool, |mut cbuf| {
            let index = cbuf.index();

            let force_record = subpasses.iter_mut().enumerate().fold(
                false,
                |force_record, (subpass_index, subpass)| {
                    subpass
                        .groups
                        .iter_mut()
                        .fold(force_record, |force_record, group| {
                            group
                                .prepare(
                                    factory,
                                    queue.id(),
                                    index,
                                    gfx_hal::pass::Subpass {
                                        index: subpass_index,
                                        main_pass: &render_pass,
                                    },
                                    aux,
                                )
                                .force_record()
                                || force_record
                        })
                },
            );

            if force_record {
                cbuf = CirqueRef::Initial(cbuf.or_reset(|cbuf| cbuf.reset()));
            }

            cbuf.or_init(|cbuf| {
                let mut cbuf = cbuf.begin(MultiShot(NoSimultaneousUse), ());
                let mut encoder = cbuf.encoder();

                if let Some(barriers) = &acquire {
                    encoder.execute_commands(std::iter::once(&barriers.submit));
                }

                let area = gfx_hal::pso::Rect {
                    x: 0,
                    y: 0,
                    w: *framebuffer_width as _,
                    h: *framebuffer_height as _,
                };

                let mut pass_encoder =
                    encoder.begin_render_pass_inline(&render_pass, framebuffer, area, &clears);

                subpasses
                    .iter_mut()
                    .enumerate()
                    .for_each(|(subpass_index, subpass)| {
                        subpass.groups.iter_mut().for_each(|group| {
                            group.draw_inline(
                                pass_encoder.reborrow(),
                                index,
                                gfx_hal::pass::Subpass {
                                    index: subpass_index,
                                    main_pass: &render_pass,
                                },
                                aux,
                            )
                        })
                    });

                drop(pass_encoder);

                if let Some(barriers) = &release {
                    encoder.execute_commands(std::iter::once(&barriers.submit));
                }
                cbuf.finish()
            })
        });

        queue.submit(
            Some(
                Submission::new()
                    .submits(Some(submit))
                    .wait(waits.iter().cloned())
                    .signal(signals.iter().cloned()),
            ),
            fence,
        );
    }

    unsafe fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &T) {
        self.common.dispose(factory, aux);
        factory.device().destroy_framebuffer(self.framebuffer);
    }
}

fn common_layout(acc: Layout, layout: Layout) -> Layout {
    match (acc, layout) {
        (Layout::Undefined, layout) => layout,
        (left, right) if left == right => left,
        (Layout::DepthStencilReadOnlyOptimal, Layout::DepthStencilAttachmentOptimal) => {
            Layout::DepthStencilAttachmentOptimal
        }
        (Layout::DepthStencilAttachmentOptimal, Layout::DepthStencilReadOnlyOptimal) => {
            Layout::DepthStencilAttachmentOptimal
        }
        (_, _) => Layout::General,
    }
}
