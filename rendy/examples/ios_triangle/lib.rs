//!
//! The mighty triangle example, used in an iOS app.
//! This examples shows a colored triangle on a white background.
//! Nothing fancy. Just to prove that `rendy` works.
//!

use rendy::{
    command::{Families, QueueId, RenderPassEncoder},
    factory::{Config, Factory},
    graph::{render::*, Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage},
    hal,
    memory::Dynamic,
    mesh::PosColor,
    resource::{Buffer, BufferInfo, DescriptorSetLayout, Escape, Handle},
    shader::{ShaderKind, SourceLanguage, SourceShaderInfo, SpirvShader},
    wsi::winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
};

#[cfg(feature = "spirv-reflection")]
use rendy::shader::SpirvReflection;

#[cfg(not(feature = "spirv-reflection"))]
use rendy::mesh::AsVertex;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangle/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangle/shader.frag").into(),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

#[cfg(feature = "spirv-reflection")]
lazy_static::lazy_static! {
    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}

#[derive(Debug, Default)]
struct TriangleRenderPipelineDesc;

#[derive(Debug)]
struct TriangleRenderPipeline<B: hal::Backend> {
    vertex: Option<Escape<Buffer<B>>>,
}

impl<B, T> SimpleGraphicsPipelineDesc<B, T> for TriangleRenderPipelineDesc
where
    B: hal::Backend,
    T: ?Sized,
{
    type Pipeline = TriangleRenderPipeline<B>;

    fn depth_stencil(&self) -> Option<hal::pso::DepthStencilDesc> {
        None
    }

    fn load_shader_set(&self, factory: &mut Factory<B>, _aux: &T) -> rendy_shader::ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        #[cfg(feature = "spirv-reflection")]
        return vec![SHADER_REFLECTION
            .attributes_range(..)
            .unwrap()
            .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)];

        #[cfg(not(feature = "spirv-reflection"))]
        return vec![PosColor::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)];
    }

    fn build<'a>(
        self,
        _ctx: &GraphContext<B>,
        _factory: &mut Factory<B>,
        _queue: QueueId,
        _aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<TriangleRenderPipeline<B>, gfx_hal::pso::CreationError> {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert!(set_layouts.is_empty());

        Ok(TriangleRenderPipeline { vertex: None })
    }
}

impl<B, T> SimpleGraphicsPipeline<B, T> for TriangleRenderPipeline<B>
where
    B: hal::Backend,
    T: ?Sized,
{
    type Desc = TriangleRenderPipelineDesc;

    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        _index: usize,
        _aux: &T,
    ) -> PrepareResult {
        if self.vertex.is_none() {
            #[cfg(feature = "spirv-reflection")]
            let vbuf_size = SHADER_REFLECTION.attributes_range(..).unwrap().stride as u64 * 3;

            #[cfg(not(feature = "spirv-reflection"))]
            let vbuf_size = PosColor::vertex().stride as u64 * 3;

            let mut vbuf = factory
                .create_buffer(
                    BufferInfo {
                        size: vbuf_size,
                        usage: hal::buffer::Usage::VERTEX,
                    },
                    Dynamic,
                )
                .unwrap();

            unsafe {
                // Fresh buffer.
                factory
                    .upload_visible_buffer(
                        &mut vbuf,
                        0,
                        &[
                            PosColor {
                                position: [0.0, -0.5, 0.0].into(),
                                color: [1.0, 0.0, 0.0, 1.0].into(),
                            },
                            PosColor {
                                position: [0.5, 0.5, 0.0].into(),
                                color: [0.0, 1.0, 0.0, 1.0].into(),
                            },
                            PosColor {
                                position: [-0.5, 0.5, 0.0].into(),
                                color: [0.0, 0.0, 1.0, 1.0].into(),
                            },
                        ],
                    )
                    .unwrap();
            }

            self.vertex = Some(vbuf);
        }

        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        _layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        _index: usize,
        _aux: &T,
    ) {
        let vbuf = self.vertex.as_ref().unwrap();
        unsafe {
            encoder.bind_vertex_buffers(0, Some((vbuf.raw(), 0)));
            encoder.draw(0..3, 0..1);
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &T) {}
}

fn run(
    event_loop: EventLoop<()>,
    mut factory: Factory<Backend>,
    mut families: Families<Backend>,
    graph: Graph<Backend, ()>,
) {
    let started = std::time::Instant::now();

    let mut frame = 0u64;
    let mut elapsed = started.elapsed();
    let mut graph = Some(graph);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::EventsCleared => {
                factory.maintain(&mut families);
                if let Some(ref mut graph) = graph {
                    graph.run(&mut factory, &mut families, &());
                    frame += 1;
                }

                elapsed = started.elapsed();
                if elapsed >= std::time::Duration::new(5, 0) {
                    *control_flow = ControlFlow::Exit
                }
            }
            _ => {}
        }

        if *control_flow == ControlFlow::Exit && graph.is_some() {
            let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

            log::info!(
                "Elapsed: {:?}. Frames: {}. FPS: {}",
                elapsed,
                frame,
                frame * 1_000_000_000 / elapsed_ns
            );

            graph.take().unwrap().dispose(&mut factory, &());
        }
    });
}

#[no_mangle]
pub extern "C" fn run_app() {
    env_logger::Builder::from_default_env()
        .filter_module("triangle", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let (mut factory, mut families): (Factory<Backend>, _) = rendy::factory::init(config).unwrap();

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)
        .unwrap();

    let surface = factory.create_surface(&window);

    let mut graph_builder = GraphBuilder::<Backend, ()>::new();

    graph_builder.add_node(
        TriangleRenderPipeline::builder()
            .into_subpass()
            .with_color_surface()
            .into_pass()
            .with_surface(
                surface,
                Some(hal::command::ClearValue {
                    color: hal::command::ClearColor {
                        float32: [1.0, 1.0, 1.0, 1.0],
                    },
                }),
            ),
    );

    let graph = graph_builder
        .build(&mut factory, &mut families, &())
        .unwrap();

    run(event_loop, factory, families, graph);
}
