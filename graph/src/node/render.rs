use crate::{
    chain,
    command::{
        ExecutableState, Graphics, IndividualReset, Submit, PrimaryLevel,
    },
    factory::Factory,
    frame::{Frames, cirque::{CommandCirque, CirqueEncoder, CirqueRenderPassInlineEncoder}},
    node::{Node, NodeBuffer, NodeBuilder, NodeDesc, NodeImage},
    resource::{Buffer, Image},
};

/// Set layout
#[derive(Clone, Debug, Default)]
pub struct SetLayout {
    pub bindings: Vec<gfx_hal::pso::DescriptorSetLayoutBinding>,
}

/// Pipeline layout
#[derive(Clone, Debug)]
pub struct Layout {
    pub sets: Vec<SetLayout>,
    pub push_constants: Vec<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>,
}

/// Pipeline info
#[derive(Clone, Debug)]
pub struct Pipeline {
    pub layout: usize,
    pub vertices: Vec<(
        Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>,
        gfx_hal::pso::ElemStride,
    )>,
    pub colors: Vec<gfx_hal::pso::ColorBlendDesc>,
    pub depth_stencil: gfx_hal::pso::DepthStencilDesc,
}

/// Render pass node.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct RenderPassNode<B: gfx_hal::Backend, R> {
    pass: R,

    extent: gfx_hal::image::Extent,

    render_pass: B::RenderPass,
    pipeline_layouts: Vec<B::PipelineLayout>,
    set_layouts: Vec<Vec<B::DescriptorSetLayout>>,
    graphics_pipelines: Vec<B::GraphicsPipeline>,

    views: Vec<B::ImageView>,
    framebuffer: B::Framebuffer,
    clears: Vec<gfx_hal::command::ClearValueRaw>,

    command_cirque: CommandCirque<B, Graphics>,
    relevant: relevant::Relevant,
}


/// Render pass.
pub trait RenderPass<B, T>: std::fmt::Debug + Send + Sync + 'static
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    /// Pass name.
    fn name() -> &'static str;

    /// Number of images to sample.
    fn sampled() -> usize {
        0
    }

    /// Number of images to use as storage.
    fn storage() -> usize {
        0
    }

    /// Number of color output images.
    fn colors() -> usize {
        1
    }

    /// Is depth image used.
    fn depth() -> bool {
        false
    }

    /// Pipeline layouts
    fn layouts() -> Vec<Layout> {
        vec![Layout {
            sets: Vec::new(),
            push_constants: Vec::new(),
        }]
    }

    /// Graphics pipelines
    fn pipelines() -> Vec<Pipeline> {
        vec![Pipeline {
            layout: 0,
            vertices: Vec::new(),
            colors: (0..Self::colors())
                .map(|_| {
                    gfx_hal::pso::ColorBlendDesc(
                        gfx_hal::pso::ColorMask::ALL,
                        gfx_hal::pso::BlendState::ALPHA,
                    )
                }).collect(),
            depth_stencil: if Self::depth() {
                gfx_hal::pso::DepthStencilDesc {
                    depth: gfx_hal::pso::DepthTest::On {
                        fun: gfx_hal::pso::Comparison::LessEqual,
                        write: true,
                    },
                    depth_bounds: false,
                    stencil: gfx_hal::pso::StencilTest::Off,
                }
            } else {
                gfx_hal::pso::DepthStencilDesc::default()
            },
        }]
    }

    /// Create `NodeBuilder` for this node.
    fn builder() -> NodeBuilder<B, T>
    where
        Self: Sized,
    {
        NodeBuilder::new(Box::<std::marker::PhantomData<Self>>::new(std::marker::PhantomData))
    }

    /// Load shader set.
    /// This function should create required shader modules and fill `GraphicsShaderSet` structure.
    ///
    /// # Parameters
    ///
    /// `storage`   - vector where this function can store loaded modules to give them required lifetime.
    ///
    /// `factory`    - `Device<B>` implementation. `B::Device` or wrapper.
    ///
    /// `aux`       - auxiliary data container. May be anything the implementation desires.
    ///
    fn load_shader_sets<'a>(
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        aux: &mut T,
    ) -> Vec<gfx_hal::pso::GraphicsShaderSet<'a, B>>;

    /// Build pass instance.
    fn build(
        sampled: &[B::ImageView],
        storage: &[B::ImageView],
        factory: &mut Factory<B>,
        aux: &mut T,
    ) -> Self;

    /// Prepare to record drawing commands.
    /// 
    /// Should return true if commands must be re-recorded.
    fn prepare(&mut self, sets: &[impl AsRef<[B::DescriptorSetLayout]>], factory: &mut Factory<B>, aux: &T) -> bool {
        false
    }

    /// Record drawing commands to the command buffer provided.
    fn draw(
        &mut self,
        layouts: &[B::PipelineLayout],
        pipelines: &[B::GraphicsPipeline],
        encoder: &mut CirqueRenderPassInlineEncoder<'_, B>,
        aux: &T,
    );

    /// Dispose of the pass.
    fn dispose(self, factory: &mut Factory<B>, aux: &mut T);
}

/// Overall description for node.
impl<B, T, R> NodeDesc<B, T> for std::marker::PhantomData<R>
where
    B: gfx_hal::Backend,
    T: ?Sized,
    R: RenderPass<B, T>,
{
    type Node = RenderPassNode<B, R>;

    fn buffers(&self) -> Vec<chain::BufferState> {
        Vec::new()
    }

    fn images(&self) -> Vec<chain::ImageState> {
        let sampled = (0..R::sampled()).map(|_| chain::ImageState {
            usage: gfx_hal::image::Usage::SAMPLED,
            access: gfx_hal::image::Access::SHADER_READ,
            layout: gfx_hal::image::Layout::ShaderReadOnlyOptimal,
            stages: all_graphics_shaders_stages(),
        });
        let storage = (0..R::storage()).map(|_| chain::ImageState {
            usage: gfx_hal::image::Usage::STORAGE,
            access: gfx_hal::image::Access::SHADER_READ,
            layout: gfx_hal::image::Layout::ShaderReadOnlyOptimal,
            stages: all_graphics_shaders_stages(),
        });
        let colors = (0..R::colors()).map(|_| chain::ImageState {
            usage: gfx_hal::image::Usage::COLOR_ATTACHMENT,
            access: gfx_hal::image::Access::COLOR_ATTACHMENT_READ
                | gfx_hal::image::Access::COLOR_ATTACHMENT_WRITE,
            layout: gfx_hal::image::Layout::ColorAttachmentOptimal,
            stages: gfx_hal::pso::PipelineStage::COLOR_ATTACHMENT_OUTPUT,
        });
        let depth = if R::depth() {
            Some(chain::ImageState {
                usage: gfx_hal::image::Usage::DEPTH_STENCIL_ATTACHMENT,
                access: gfx_hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ
                    | gfx_hal::image::Access::DEPTH_STENCIL_ATTACHMENT_WRITE,
                layout: gfx_hal::image::Layout::DepthStencilAttachmentOptimal,
                stages: gfx_hal::pso::PipelineStage::EARLY_FRAGMENT_TESTS
                    | gfx_hal::pso::PipelineStage::LATE_FRAGMENT_TESTS,
            })
        } else {
            None
        };

        sampled.chain(storage).chain(colors).chain(depth).collect()
    }

    fn build<'a>(
        &self,
        factory: &mut Factory<B>,
        aux: &mut T,
        buffers: impl IntoIterator<Item = NodeBuffer<'a, B>>,
        images: impl IntoIterator<Item = NodeImage<'a, B>>,
        family: gfx_hal::queue::QueueFamilyId,
    ) -> Result<Self::Node, failure::Error> {
        log::trace!("Creating RenderPass instance for '{}'", R::name());

        assert!(buffers.into_iter().all(|_| false));
        let images: Vec<_> = images.into_iter().collect();

        assert_eq!(
            R::sampled() + R::storage() + R::colors() + R::depth() as usize,
            images.len()
        );

        let color = |index| &images[R::sampled() + R::storage() + index];
        let depth = || &images[R::sampled() + R::storage() + R::colors()];

        let render_pass: B::RenderPass = {
            let attachments = (0..R::colors())
                .map(|index| gfx_hal::pass::Attachment {
                    format: Some(color(index).image.format()),
                    ops: gfx_hal::pass::AttachmentOps {
                        load: if color(index).clear.is_some() {
                            gfx_hal::pass::AttachmentLoadOp::Clear
                        } else {
                            gfx_hal::pass::AttachmentLoadOp::Load
                        },
                        store: gfx_hal::pass::AttachmentStoreOp::Store,
                    },
                    stencil_ops: gfx_hal::pass::AttachmentOps::DONT_CARE,
                    layouts: {
                        let layout = color(index).state.layout;
                        let from = if color(index).clear.is_some() {
                            gfx_hal::image::Layout::Undefined
                        } else {
                            layout
                        };
                        from..layout
                    },
                    samples: 1,
                }).chain(if R::depth() {
                    Some(gfx_hal::pass::Attachment {
                        format: Some(depth().image.format()),
                        ops: gfx_hal::pass::AttachmentOps {
                            load: if depth().clear.is_some() {
                                gfx_hal::pass::AttachmentLoadOp::Clear
                            } else {
                                gfx_hal::pass::AttachmentLoadOp::Load
                            },
                            store: gfx_hal::pass::AttachmentStoreOp::Store,
                        },
                        stencil_ops: gfx_hal::pass::AttachmentOps::DONT_CARE,
                        layouts: {
                            let layout = depth().state.layout;
                            let from = if depth().clear.is_some() {
                                gfx_hal::image::Layout::Undefined
                            } else {
                                layout
                            };
                            from..layout
                        },
                        samples: 1,
                    })
                } else {
                    None
                });

            let colors = (0..R::colors())
                .map(|index| (index, color(index).state.layout))
                .collect::<Vec<_>>();
            let depth = if R::depth() {
                Some((R::colors(), depth().state.layout))
            } else {
                None
            };

            let subpass = gfx_hal::pass::SubpassDesc {
                colors: &colors,
                depth_stencil: depth.as_ref(),
                inputs: &[],
                resolves: &[],
                preserves: &[],
            };

            let result = gfx_hal::Device::create_render_pass(
                factory.device(),
                attachments,
                Some(subpass),
                std::iter::empty::<gfx_hal::pass::SubpassDependency>(),
            ).unwrap();

            log::trace!("RenderPass instance created for '{}'", R::name());
            result
        };

        log::trace!("Collect clears for '{}'", R::name());

        let clears: Vec<_> = (0..R::colors())
            .map(|index| {
                color(index)
                    .clear
                    .unwrap_or(gfx_hal::command::ClearValue::Color(
                        gfx_hal::command::ClearColor::Float([0.3, 0.7, 0.9, 1.0]),
                    ))
            }).chain(if R::depth() { depth().clear } else { None })
            .map(Into::into)
            .collect();

        log::trace!("Create views for '{}'", R::name());

        let mut extent = None;

        let views: Vec<B::ImageView> = images
            .iter()
            .enumerate()
            .map(|(i, image)| {
                if i >= R::sampled() + R::storage() {
                    // This is color or depth attachment.
                    assert!(
                        match image.image.kind() {
                            gfx_hal::image::Kind::D2(_, _, _, _) => true,
                            _ => false,
                        },
                        "Attachments must be D2 images"
                    );

                    assert!(
                        extent.map_or(true, |e| e == image.image.kind().extent()),
                        "All attachments must have same `Extent`"
                    );
                    extent = Some(image.image.kind().extent());
                }

                let view_kind = match image.image.kind() {
                    gfx_hal::image::Kind::D1(_, _) => gfx_hal::image::ViewKind::D1,
                    gfx_hal::image::Kind::D2(_, _, _, _) => gfx_hal::image::ViewKind::D2,
                    gfx_hal::image::Kind::D3(_, _, _) => gfx_hal::image::ViewKind::D3,
                };

                let subresource_range = gfx_hal::image::SubresourceRange {
                    aspects: image.image.format().surface_desc().aspects,
                    levels: 0..1,
                    layers: 0..1,
                };

                gfx_hal::Device::create_image_view(
                    factory.device(),
                    image.image.raw(),
                    view_kind,
                    image.image.format(),
                    gfx_hal::format::Swizzle::NO,
                    subresource_range.clone(),
                )
            }).collect::<Result<_, _>>()?;

        let extent = extent.unwrap_or(gfx_hal::image::Extent {
            width: 1,
            height: 1,
            depth: 1,
        });

        let rect = gfx_hal::pso::Rect {
            x: 0,
            y: 0,
            w: extent.width as _,
            h: extent.height as _,
        };

        log::trace!("Creating layouts for '{}'", R::name());

        let (pipeline_layouts, set_layouts): (Vec<_>, Vec<_>) = R::layouts()
            .into_iter()
            .map(|layout| {
                let set_layouts = layout
                    .sets
                    .into_iter()
                    .map(|set| {
                        gfx_hal::Device::create_descriptor_set_layout(
                            factory.device(),
                            set.bindings,
                            std::iter::empty::<B::Sampler>(),
                        )
                    }).collect::<Result<Vec<_>, _>>()?;
                let pipeline_layout = gfx_hal::Device::create_pipeline_layout(
                    factory.device(),
                    &set_layouts,
                    layout.push_constants,
                )?;
                Ok((pipeline_layout, set_layouts))
            }).collect::<Result<Vec<_>, failure::Error>>()?
            .into_iter()
            .unzip();

        log::trace!("Creating graphics pipelines for '{}'", R::name());

        let graphics_pipelines = {
            let mut shaders = Vec::new();

            let pipelines = R::pipelines();

            let descs = pipelines
                .iter()
                .zip(R::load_shader_sets(&mut shaders, factory, aux))
                .enumerate()
                .map(|(index, (pipeline, shader_set))| {
                    assert_eq!(pipeline.colors.len(), R::colors());
                    // assert_eq!(pipeline.depth_stencil.is_some(), R::depth());

                    let mut vertex_buffers = Vec::new();
                    let mut attributes = Vec::new();

                    for &(ref elemets, stride) in &pipeline.vertices {
                        push_vertex_desc(elemets, stride, &mut vertex_buffers, &mut attributes);
                    }

                    gfx_hal::pso::GraphicsPipelineDesc {
                        shaders: shader_set,
                        rasterizer: gfx_hal::pso::Rasterizer::FILL,
                        vertex_buffers,
                        attributes,
                        input_assembler: gfx_hal::pso::InputAssemblerDesc {
                            primitive: gfx_hal::Primitive::TriangleList,
                            primitive_restart: gfx_hal::pso::PrimitiveRestart::Disabled,
                        },
                        blender: gfx_hal::pso::BlendDesc {
                            logic_op: None,
                            targets: pipeline.colors.clone(),
                        },
                        depth_stencil: pipeline.depth_stencil,
                        multisampling: None,
                        baked_states: gfx_hal::pso::BakedStates {
                            viewport: Some(gfx_hal::pso::Viewport {
                                rect,
                                depth: 0.0..1.0,
                            }),
                            scissor: Some(rect),
                            blend_color: None,
                            depth_bounds: None,
                        },
                        layout: &pipeline_layouts[pipeline.layout],
                        subpass: gfx_hal::pass::Subpass {
                            index: 0,
                            main_pass: &render_pass,
                        },
                        flags: if index == 0 && pipelines.len() > 1 {
                            gfx_hal::pso::PipelineCreationFlags::ALLOW_DERIVATIVES
                        } else {
                            gfx_hal::pso::PipelineCreationFlags::empty()
                        },
                        parent: gfx_hal::pso::BasePipeline::None,
                    }
                });

            let pipelines =
                gfx_hal::Device::create_graphics_pipelines(factory.device(), descs, None)
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
            log::trace!("Graphics pipeline created for '{}'", R::name());
            pipelines
        };

        let framebuffer = gfx_hal::Device::create_framebuffer(
            factory.device(),
            &render_pass,
            views.iter(),
            extent,
        )?;

        let pass = R::build(
            &views[..R::sampled()],
            &views[R::sampled()..R::sampled() + R::storage()],
            factory,
            aux,
        );

        let command_cirque = factory.create_command_pool(family, IndividualReset)?
                .with_capability()
                .map(|pool| CommandCirque::new(pool, PrimaryLevel))
                .expect("Graph must specify family that supports `Graphics`");

        Ok(RenderPassNode {
            pass,
            extent,
            render_pass,
            pipeline_layouts,
            set_layouts,
            graphics_pipelines,
            views,
            framebuffer,
            clears,
            command_cirque,
            relevant: relevant::Relevant,
        })
    }
}

impl<B, T, R> Node<B, T> for RenderPassNode<B, R>
where
    B: gfx_hal::Backend,
    T: ?Sized,
    R: RenderPass<B, T>,
{
    type Capability = Graphics;
    type Desc = std::marker::PhantomData<R>;

    fn run<'a>(
        &mut self,
        factory: &mut Factory<B>,
        aux: &mut T,
        frames: &'a Frames<B>,
    ) -> Submit<'a, B> {
        let redraw = self.pass.prepare(&self.set_layouts, factory, aux);

        let encoder = unsafe {
            /// Graph supplies same `frames`.
            self.command_cirque.get(frames)
        };

        let mut recording = match encoder {
            either::Left(executable) => {
                if !redraw {
                    return executable.submit();
                }
                executable.reset()
            }
            either::Right(initial) => initial,
        }.begin();

        {
            let area = gfx_hal::pso::Rect {
                x: 0,
                y: 0,
                w: self.extent.width as _,
                h: self.extent.height as _,
            };

            let mut pass_encoder = unsafe {
                recording.begin_render_pass_inline(
                    &self.render_pass,
                    &self.framebuffer,
                    area,
                    &self.clears,
                )
            };

            self.pass.draw(
                &self.pipeline_layouts,
                &self.graphics_pipelines,
                &mut pass_encoder,
                aux,
            );
        }

        recording.finish().submit()
    }

    fn dispose(self, factory: &mut Factory<B>, aux: &mut T) {
        unimplemented!()
    }
}

fn all_graphics_shaders_stages() -> gfx_hal::pso::PipelineStage {
    gfx_hal::pso::PipelineStage::VERTEX_SHADER
        // | gfx_hal::pso::PipelineStage::DOMAIN_SHADER
        // | gfx_hal::pso::PipelineStage::HULL_SHADER
        // | gfx_hal::pso::PipelineStage::GEOMETRY_SHADER
        | gfx_hal::pso::PipelineStage::FRAGMENT_SHADER
}

fn push_vertex_desc(
    elements: &[gfx_hal::pso::Element<gfx_hal::format::Format>],
    stride: gfx_hal::pso::ElemStride,
    vertex_buffers: &mut Vec<gfx_hal::pso::VertexBufferDesc>,
    attributes: &mut Vec<gfx_hal::pso::AttributeDesc>,
) {
    let index = vertex_buffers.len() as gfx_hal::pso::BufferIndex;

    vertex_buffers.push(gfx_hal::pso::VertexBufferDesc {
        binding: 0,
        stride,
        rate: 0,
    });

    let mut location = attributes.last().map(|a| a.location + 1).unwrap_or(0);
    for &element in elements {
        attributes.push(gfx_hal::pso::AttributeDesc {
            location,
            binding: index,
            element,
        });
        location += 1;
    }
}
