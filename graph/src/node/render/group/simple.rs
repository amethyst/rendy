use hal::{device::Device as _, Backend};
use rendy_core::hal;
pub use rendy_core::types::{Layout, SetLayout};

use super::{RenderGroup, RenderGroupDesc};
use crate::{
    command::{QueueId, RenderPassEncoder},
    factory::Factory,
    graph::GraphContext,
    node::{render::PrepareResult, BufferAccess, DescBuilder, ImageAccess, NodeBuffer, NodeImage},
    resource::{DescriptorSetLayout, Handle},
};

/// Pipeline info
#[derive(Clone, Debug)]
pub struct Pipeline {
    /// Layout for pipeline.
    pub layout: Layout,

    /// Vertex input for pipeline.
    pub vertices: Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )>,

    /// Colors for pipeline.
    pub colors: Vec<hal::pso::ColorBlendDesc>,

    /// Depth stencil for pipeline.
    pub depth_stencil: hal::pso::DepthStencilDesc,

    /// Rasterizer for pipeline.
    pub rasterizer: hal::pso::Rasterizer,

    /// Primitive to use in the input assembler.
    pub input_assembler_desc: hal::pso::InputAssemblerDesc,
}

/// Descriptor for simple graphics pipeline implementation.
pub trait SimpleGraphicsPipelineDesc<B: Backend, T: ?Sized>: std::fmt::Debug {
    /// Simple graphics pipeline implementation
    type Pipeline: SimpleGraphicsPipeline<B, T>;

    /// Make simple render group builder.
    fn builder(self) -> DescBuilder<B, T, SimpleRenderGroupDesc<Self>>
    where
        Self: Sized,
    {
        SimpleRenderGroupDesc { inner: self }.builder()
    }

    /// Get set or buffer resources the node uses.
    fn buffers(&self) -> Vec<BufferAccess> {
        Vec::new()
    }

    /// Get set or image resources the node uses.
    fn images(&self) -> Vec<ImageAccess> {
        Vec::new()
    }

    /// Color blend descs.
    fn colors(&self) -> Vec<hal::pso::ColorBlendDesc> {
        vec![hal::pso::ColorBlendDesc {
            mask: hal::pso::ColorMask::ALL,
            blend: Some(hal::pso::BlendState::ALPHA),
        }]
    }

    /// Depth stencil desc.
    fn depth_stencil(&self) -> Option<hal::pso::DepthStencilDesc> {
        Some(hal::pso::DepthStencilDesc {
            depth: Some(hal::pso::DepthTest {
                fun: hal::pso::Comparison::Less,
                write: true,
            }),
            depth_bounds: false,
            stencil: None,
        })
    }

    /// Rasterizer desc.
    fn rasterizer(&self) -> hal::pso::Rasterizer {
        hal::pso::Rasterizer::FILL
    }

    /// Get vertex input.
    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        Vec::new()
    }

    /// Layout for graphics pipeline
    /// Default implementation for `pipeline` will use this.
    fn layout(&self) -> Layout {
        Layout::default()
    }

    /// Returns the InputAssemblerDesc. Defaults to a TriangleList with Restart disabled, can be overriden.
    fn input_assembler(&self) -> hal::pso::InputAssemblerDesc {
        hal::pso::InputAssemblerDesc {
            primitive: hal::pso::Primitive::TriangleList,
            with_adjacency: false,
            restart_index: None,
        }
    }

    /// Graphics pipelines
    fn pipeline(&self) -> Pipeline {
        Pipeline {
            layout: Layout::default(),
            vertices: Vec::new(),
            colors: self.colors(),
            depth_stencil: self.depth_stencil().unwrap_or_default(),
            rasterizer: self.rasterizer(),
            input_assembler_desc: self.input_assembler(),
        }
    }

    /// Load shader set.
    /// This function should utilize the provided `ShaderSetBuilder` reflection class and return the compiled `ShaderSet`.
    ///
    /// # Parameters
    ///
    /// `factory`   - factory to create shader modules.
    ///
    /// `aux`       - auxiliary data container. May be anything the implementation desires.
    fn load_shader_set(&self, factory: &mut Factory<B>, aux: &T) -> rendy_shader::ShaderSet<B>;

    /// Build pass instance.
    fn build(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Self::Pipeline, hal::pso::CreationError>;
}

/// Simple render pipeline.
pub trait SimpleGraphicsPipeline<B: Backend, T: ?Sized>:
    std::fmt::Debug + Sized + Send + Sync + 'static
{
    /// This pipeline descriptor.
    type Desc: SimpleGraphicsPipelineDesc<B, T, Pipeline = Self>;

    /// Make simple render group builder.
    fn builder() -> DescBuilder<B, T, SimpleRenderGroupDesc<Self::Desc>>
    where
        Self::Desc: Default,
    {
        Self::Desc::default().builder()
    }

    /// Prepare to record drawing commands.
    ///
    /// Should return true if commands must be re-recorded.
    fn prepare(
        &mut self,
        _factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        _index: usize,
        _aux: &T,
    ) -> PrepareResult {
        PrepareResult::DrawRecord
    }

    /// Record drawing commands to the command buffer provided.
    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        encoder: RenderPassEncoder<'_, B>,
        index: usize,
        aux: &T,
    );

    /// Free all resources and destroy pipeline instance.
    fn dispose(self, factory: &mut Factory<B>, aux: &T);
}

/// Render group that consist of simple graphics pipeline.
#[derive(Debug)]
pub struct SimpleRenderGroup<B: Backend, P> {
    set_layouts: Vec<Handle<DescriptorSetLayout<B>>>,
    pipeline_layout: B::PipelineLayout,
    graphics_pipeline: B::GraphicsPipeline,
    pipeline: P,
}

/// Descriptor for simple render group.
#[derive(Debug)]
pub struct SimpleRenderGroupDesc<P: std::fmt::Debug> {
    inner: P,
}

impl<B, T, P> RenderGroupDesc<B, T> for SimpleRenderGroupDesc<P>
where
    B: Backend,
    T: ?Sized,
    P: SimpleGraphicsPipelineDesc<B, T>,
{
    fn buffers(&self) -> Vec<BufferAccess> {
        self.inner.buffers()
    }

    fn images(&self) -> Vec<ImageAccess> {
        self.inner.images()
    }

    fn colors(&self) -> usize {
        self.inner.colors().len()
    }

    fn depth(&self) -> bool {
        self.inner.depth_stencil().is_some()
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &T,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, T>>, hal::pso::CreationError> {
        let mut shader_set = self.inner.load_shader_set(factory, aux);

        let pipeline = self.inner.pipeline();

        let set_layouts = pipeline
            .layout
            .sets
            .into_iter()
            .map(|set| {
                factory
                    .create_descriptor_set_layout(set.bindings)
                    .map(Handle::from)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                shader_set.dispose(factory);
                e
            })?;

        let pipeline_layout = unsafe {
            factory.device().create_pipeline_layout(
                set_layouts.iter().map(|l| l.raw()),
                pipeline.layout.push_constants,
            )
        }
        .map_err(|e| {
            shader_set.dispose(factory);
            hal::pso::CreationError::OutOfMemory(e)
        })?;

        let mut vertex_buffers = Vec::new();
        let mut attributes = Vec::new();

        for &(ref elemets, stride, rate) in &pipeline.vertices {
            push_vertex_desc(elemets, stride, rate, &mut vertex_buffers, &mut attributes);
        }

        let rect = hal::pso::Rect {
            x: 0,
            y: 0,
            w: framebuffer_width as i16,
            h: framebuffer_height as i16,
        };

        let shaders = match shader_set.raw() {
            Err(_) => {
                shader_set.dispose(factory);
                return Err(hal::pso::CreationError::Other);
            }
            Ok(s) => s,
        };

        let graphics_pipeline = unsafe {
            factory.device().create_graphics_pipelines(
                Some(hal::pso::GraphicsPipelineDesc {
                    shaders,
                    rasterizer: pipeline.rasterizer,
                    vertex_buffers,
                    attributes,
                    input_assembler: pipeline.input_assembler_desc,
                    blender: hal::pso::BlendDesc {
                        logic_op: None,
                        targets: pipeline.colors.clone(),
                    },
                    depth_stencil: pipeline.depth_stencil,
                    multisampling: None,
                    baked_states: hal::pso::BakedStates {
                        viewport: Some(hal::pso::Viewport {
                            rect,
                            depth: 0.0..1.0,
                        }),
                        scissor: Some(rect),
                        blend_color: None,
                        depth_bounds: None,
                    },
                    layout: &pipeline_layout,
                    subpass,
                    flags: hal::pso::PipelineCreationFlags::empty(),
                    parent: hal::pso::BasePipeline::None,
                }),
                None,
            )
        }
        .remove(0)
        .map_err(|e| {
            shader_set.dispose(factory);
            e
        })?;

        let pipeline = self
            .inner
            .build(ctx, factory, queue, aux, buffers, images, &set_layouts)
            .map_err(|e| {
                shader_set.dispose(factory);
                e
            })?;

        shader_set.dispose(factory);

        Ok(Box::new(SimpleRenderGroup::<B, _> {
            set_layouts,
            pipeline_layout,
            graphics_pipeline,
            pipeline,
        }))
    }
}

impl<B, T, P> RenderGroup<B, T> for SimpleRenderGroup<B, P>
where
    B: Backend,
    T: ?Sized,
    P: SimpleGraphicsPipeline<B, T>,
{
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        aux: &T,
    ) -> PrepareResult {
        self.pipeline
            .prepare(factory, queue, &self.set_layouts, index, aux)
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        aux: &T,
    ) {
        encoder.bind_graphics_pipeline(&self.graphics_pipeline);
        self.pipeline
            .draw(&self.pipeline_layout, encoder, index, aux);
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &T) {
        self.pipeline.dispose(factory, aux);

        unsafe {
            factory
                .device()
                .destroy_graphics_pipeline(self.graphics_pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
            drop(self.set_layouts);
        }
    }
}

fn push_vertex_desc(
    elements: &[hal::pso::Element<hal::format::Format>],
    stride: hal::pso::ElemStride,
    rate: hal::pso::VertexInputRate,
    vertex_buffers: &mut Vec<hal::pso::VertexBufferDesc>,
    attributes: &mut Vec<hal::pso::AttributeDesc>,
) {
    let index = vertex_buffers.len() as hal::pso::BufferIndex;

    vertex_buffers.push(hal::pso::VertexBufferDesc {
        binding: index,
        stride,
        rate,
    });

    let mut location = attributes.last().map_or(0, |a| a.location + 1);
    for &element in elements {
        attributes.push(hal::pso::AttributeDesc {
            location,
            binding: index,
            element,
        });
        location += 1;
    }
}
