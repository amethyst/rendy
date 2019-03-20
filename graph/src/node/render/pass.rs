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
        node::{
            gfx_acquire_barriers, gfx_release_barriers, is_metal,
            render::group::{RenderGroup, RenderGroupBuilder},
            BufferAccess, DynNode, ImageAccess, NodeBuffer, NodeBuilder, NodeImage,
        },
        BufferId, ImageId, NodeId,
    },
    gfx_hal::{image::Layout, Backend, Device},
    std::{cmp::min, collections::HashMap},
};

#[derive(derivative::Derivative)]
#[derivative(Default(bound = ""), Debug(bound = ""))]
pub struct SubpassBuilder<B: Backend, T: ?Sized> {
    groups: Vec<Box<dyn RenderGroupBuilder<B, T>>>,
    inputs: Vec<ImageId>,
    colors: Vec<ImageId>,
    depth_stencil: Option<ImageId>,
    dependencies: Vec<NodeId>,
}

impl<B, T> SubpassBuilder<B, T>
where
    B: Backend,
    T: ?Sized,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_group<R>(&mut self, group: R) -> &mut Self
    where
        R: RenderGroupBuilder<B, T> + 'static,
    {
        self.groups.push(Box::new(group));
        self
    }

    pub fn with_group<R>(mut self, group: R) -> Self
    where
        R: RenderGroupBuilder<B, T> + 'static,
    {
        self.add_group(group);
        self
    }

    pub fn add_input(&mut self, input: ImageId) -> &mut Self {
        self.inputs.push(input);
        self
    }

    pub fn with_input(mut self, input: ImageId) -> Self {
        self.add_input(input);
        self
    }

    pub fn add_color(&mut self, color: ImageId) -> &mut Self {
        self.colors.push(color);
        self
    }

    pub fn with_color(mut self, color: ImageId) -> Self {
        self.add_color(color);
        self
    }

    pub fn set_depth_stencil(&mut self, depth_stencil: ImageId) -> &mut Self {
        self.depth_stencil = Some(depth_stencil);
        self
    }

    pub fn with_depth_stencil(mut self, depth_stencil: ImageId) -> Self {
        self.set_depth_stencil(depth_stencil);
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

#[derive(derivative::Derivative)]
#[derivative(Default(bound = ""), Debug(bound = ""))]
pub struct RenderPassNodeBuilder<B: Backend, T: ?Sized> {
    subpasses: Vec<SubpassBuilder<B, T>>,
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

    pub fn add_subpass(&mut self, subpass: SubpassBuilder<B, T>) -> &mut Self {
        self.subpasses.push(subpass);
        self
    }

    pub fn with_subpass(mut self, subpass: SubpassBuilder<B, T>) -> Self {
        self.add_subpass(subpass);
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
            for &id in &subpass.inputs {
                let entry = attachments.entry(id).or_insert(ImageAccess {
                    layout: Layout::ShaderReadOnlyOptimal,
                    ..empty
                });
                entry.access |= gfx_hal::image::Access::INPUT_ATTACHMENT_READ;
                entry.usage |= gfx_hal::image::Usage::INPUT_ATTACHMENT;
                entry.stages |= gfx_hal::pso::PipelineStage::FRAGMENT_SHADER;
            }

            for &id in &subpass.colors {
                let entry = attachments.entry(id).or_insert(ImageAccess {
                    layout: Layout::ColorAttachmentOptimal,
                    ..empty
                });
                entry.access |= gfx_hal::image::Access::COLOR_ATTACHMENT_READ
                    | gfx_hal::image::Access::COLOR_ATTACHMENT_WRITE;
                entry.usage |= gfx_hal::image::Usage::COLOR_ATTACHMENT;
                entry.stages |= gfx_hal::pso::PipelineStage::COLOR_ATTACHMENT_OUTPUT;
            }

            if let &Some(id) = &subpass.depth_stencil {
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
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        queue: usize,
        aux: &T,
        mut buffers: Vec<NodeBuffer<'a, B>>,
        images: Vec<NodeImage<'a, B>>,
    ) -> Result<Box<dyn DynNode<B, T>>, failure::Error> {
        let mut attachment_ids: Vec<ImageId> = self
            .subpasses
            .iter()
            .flat_map(|subpass| {
                subpass
                    .inputs
                    .iter()
                    .chain(subpass.colors.iter())
                    .chain(subpass.depth_stencil.as_ref())
                    .cloned()
            })
            .collect();

        attachment_ids.sort();
        attachment_ids.dedup();

        let (mut attachments, mut images): (Vec<_>, _) = images
            .into_iter()
            .partition(|image| attachment_ids.binary_search(&image.id).is_ok());

        let find_attachment = |id: ImageId| {
            attachments
                .iter()
                .find(|a| a.id == id)
                .expect("Attachment image wasn't provided")
        };

        let mut framebuffer_width = u32::max_value();
        let mut framebuffer_height = u32::max_value();
        let mut framebuffer_layers = u16::max_value();

        let views: Vec<_> = attachment_ids
            .iter()
            .map(|&id| unsafe {
                let attachment = find_attachment(id);
                let extent = attachment.image.kind().extent();
                framebuffer_width = min(framebuffer_width, extent.width);
                framebuffer_height = min(framebuffer_height, extent.height);
                framebuffer_layers = min(
                    framebuffer_layers,
                    attachment.range.layers.end - attachment.range.layers.start,
                );
                factory
                    .device()
                    .create_image_view(
                        attachment.image.raw(),
                        gfx_hal::image::ViewKind::D2,
                        attachment.image.format(),
                        gfx_hal::format::Swizzle::NO,
                        attachment.range.clone(),
                    )
                    .map_err(failure::Error::from)
            })
            .collect::<Result<_, _>>()?;

        let render_pass: B::RenderPass = {
            let attachments: Vec<_> = attachment_ids
                .iter()
                .map(|&id| {
                    let attachment = find_attachment(id);
                    gfx_hal::pass::Attachment {
                        format: Some(attachment.image.format()),
                        ops: gfx_hal::pass::AttachmentOps {
                            load: if attachment.clear.is_some() {
                                gfx_hal::pass::AttachmentLoadOp::Clear
                            } else {
                                gfx_hal::pass::AttachmentLoadOp::Load
                            },
                            store: gfx_hal::pass::AttachmentStoreOp::Store,
                        },
                        stencil_ops: gfx_hal::pass::AttachmentOps::DONT_CARE,
                        layouts: {
                            let layout = attachment.layout;
                            let from = if attachment.clear.is_some() {
                                gfx_hal::image::Layout::Undefined
                            } else {
                                layout
                            };
                            from..layout
                        },
                        samples: 1,
                    }
                })
                .collect();

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
                        .map(|&id| {
                            (
                                attachment_ids.iter().position(|&a| a == id).unwrap(),
                                find_attachment(id).layout,
                            )
                        })
                        .collect(),
                    colors: subpass
                        .colors
                        .iter()
                        .map(|&id| {
                            (
                                attachment_ids.iter().position(|&a| a == id).unwrap(),
                                find_attachment(id).layout,
                            )
                        })
                        .collect(),
                    depth_stencil: subpass.depth_stencil.map(|id| {
                        (
                            attachment_ids.iter().position(|&a| a == id).unwrap(),
                            find_attachment(id).layout,
                        )
                    }),
                })
                .collect();

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
                gfx_hal::Device::create_render_pass(factory.device(), attachments, subpasses, {
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

        let framebuffer = unsafe {
            factory.device().create_framebuffer(
                &render_pass,
                &views,
                gfx_hal::image::Extent {
                    width: framebuffer_width,
                    height: framebuffer_height,
                    depth: framebuffer_layers as u32, // This is gfx-hal BUG as this parameter actually means framebuffer layers number,
                },
            )
        }?;

        log::trace!("Collect clears for render pass");

        let clears: Vec<_> = attachment_ids
            .iter()
            .filter_map(|&id| find_attachment(id).clear)
            .map(Into::into)
            .collect();

        let mut command_pool = factory
            .create_command_pool(family)?
            .with_capability()
            .expect("Graph must specify family that supports `Graphics`");

        let command_cirque = CommandCirque::new();

        let acquire = if !is_metal::<B>() {
            let (stages, barriers) = gfx_acquire_barriers(
                &buffers,
                images
                    .iter()
                    .chain(attachments.iter_mut().map(|attachment| {
                        if attachment.clear.is_some() {
                            if let Some(ref mut acquire) = &mut attachment.acquire {
                                acquire.states.start = (
                                    gfx_hal::image::Access::empty(),
                                    gfx_hal::image::Layout::Undefined,
                                );
                            }
                        }
                        &*attachment
                    })),
            );

            if !barriers.is_empty() {
                let initial = command_pool.allocate_buffers(1).pop().unwrap();
                let mut recording = initial.begin(MultiShot(SimultaneousUse), ());
                log::info!("Acquire {:?} : {:#?}", stages, barriers);
                recording.encoder().pipeline_barrier(
                    stages,
                    gfx_hal::memory::Dependencies::empty(),
                    barriers,
                );
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
            let (stages, barriers) =
                gfx_release_barriers(&buffers, images.iter().chain(attachments.iter()));

            if !barriers.is_empty() {
                let initial = command_pool.allocate_buffers(1).pop().unwrap();
                let mut recording = initial.begin(MultiShot(SimultaneousUse), ());
                log::info!("Release {:?} : {:#?}", stages, barriers);
                recording.encoder().pipeline_barrier(
                    stages,
                    gfx_hal::memory::Dependencies::empty(),
                    barriers,
                );
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

                        let mut buffers = buffers.iter_mut();
                        let mut images = images.iter_mut();

                        let buffers: Vec<_> = group
                            .buffers()
                            .into_iter()
                            .map(|(id, _)| {
                                buffers
                                    .find(|b| b.id == id)
                                    .expect("Transient buffer wasn't provided")
                                    .reborrow()
                            })
                            .collect();
                        let images: Vec<_> = group
                            .images()
                            .into_iter()
                            .map(|(id, _)| {
                                images
                                    .find(|i| i.id == id)
                                    .expect("Transient image wasn't provided")
                                    .reborrow()
                            })
                            .collect();

                        group.build(
                            factory,
                            QueueId(family.id(), queue),
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

        let node = RenderPassNode {
            subpasses,

            framebuffer_width,
            framebuffer_height,
            _framebuffer_layers: framebuffer_layers,

            render_pass,
            views,
            framebuffer,
            clears,

            command_pool,
            command_cirque,

            acquire,
            release,

            relevant: relevant::Relevant,
        };

        Ok(Box::new(node))
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
struct BarriersCommands<B: gfx_hal::Backend> {
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
pub struct RenderPassNode<B: Backend, T: ?Sized> {
    subpasses: Vec<SubpassNode<B, T>>,

    framebuffer_width: u32,
    framebuffer_height: u32,
    _framebuffer_layers: u16,

    render_pass: B::RenderPass,
    views: Vec<B::ImageView>,
    framebuffer: B::Framebuffer,
    clears: Vec<gfx_hal::command::ClearValueRaw>,

    command_pool: CommandPool<B, Graphics, IndividualReset>,
    command_cirque: CommandCirque<B, Graphics>,

    acquire: Option<BarriersCommands<B>>,
    release: Option<BarriersCommands<B>>,

    relevant: relevant::Relevant,
}

impl<B, T> DynNode<B, T> for RenderPassNode<B, T>
where
    B: Backend,
    T: ?Sized,
{
    unsafe fn run<'a>(
        &mut self,
        factory: &Factory<B>,
        queue: &mut Queue<B>,
        aux: &T,
        frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&mut Fence<B>>,
    ) {
        let RenderPassNode {
            subpasses,

            framebuffer_width,
            framebuffer_height,

            render_pass,
            framebuffer,
            clears,

            command_cirque,
            command_pool,

            acquire,
            release,
            ..
        } = self;

        let submit = command_cirque.encode(frames, command_pool, |mut cbuf| {
            let index = cbuf.index();

            let force_record = subpasses
                .iter_mut()
                .enumerate()
                .any(|(subpass_index, subpass)| {
                    subpass.groups.iter_mut().any(|group| {
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
                    })
                });

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
                    { encoder.begin_render_pass_inline(&render_pass, &framebuffer, area, &clears) };

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
        )
    }

    unsafe fn dispose(mut self: Box<Self>, factory: &mut Factory<B>, aux: &T) {
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

        factory.device().destroy_framebuffer(self.framebuffer);
        for view in self.views {
            factory.device().destroy_image_view(view);
        }
        factory.device().destroy_render_pass(self.render_pass);
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
