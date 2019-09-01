//!
//! First non-trivial example simulates miriad of quads floating in gravity field and bouce of window borders.
//! It uses compute node to move quads and simple render pass to draw them.
//!

#![cfg_attr(
    not(any(feature = "dx12", feature = "metal", feature = "vulkan")),
    allow(unused)
)]

use rendy::{
    command::{
        CommandBuffer, CommandPool, Compute, DrawCommand, ExecutableState, Families, Family,
        MultiShot, PendingState, QueueId, RenderPassEncoder, SimultaneousUse, Submit,
    },
    factory::{BufferState, Config, Factory},
    frame::Frames,
    graph::{
        gfx_acquire_barriers, gfx_release_barriers,
        present::PresentNode,
        render::{
            Layout, PrepareResult, RenderGroupBuilder, SimpleGraphicsPipeline,
            SimpleGraphicsPipelineDesc,
        },
        BufferAccess, Graph, GraphBuilder, GraphContext, Node,
        NodeBuildError, NodeBuffer, NodeDesc, NodeImage, NodeSubmittable,
    },
    hal::{self, device::Device as _},
    memory::Dynamic,
    mesh::Color,
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{SourceShaderInfo, Shader, ShaderKind, SourceLanguage, SpirvShader},
    wsi::winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::{Window, WindowBuilder},
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

#[repr(C)]
#[derive(Copy, Clone)]
struct PosVel {
    pos: [f32; 2],
    vel: [f32; 2],
}

lazy_static::lazy_static! {
    static ref RENDER_VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("render.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/render.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref RENDER_FRAGMENT: SpirvShader = SourceShaderInfo::new(
        include_str!("render.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/render.frag").into(),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref BOUNCE_COMPUTE: SpirvShader = SourceShaderInfo::new(
        include_str!("bounce.comp"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/bounce.comp").into(),
        ShaderKind::Compute,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref POSVEL_DATA: Vec<PosVel> = {
        let mut rng = rand::thread_rng();
        let uniform = rand::distributions::Uniform::new(0.0, 1.0);
        (0..QUADS)
            .map(|_index| PosVel {
                pos: [
                    rand::Rng::sample(&mut rng, uniform),
                    rand::Rng::sample(&mut rng, uniform),
                ],
                vel: [
                    rand::Rng::sample(&mut rng, uniform),
                    rand::Rng::sample(&mut rng, uniform),
                ],
            })
            .collect()
    };

    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*RENDER_VERTEX).unwrap()
        .with_fragment(&*RENDER_FRAGMENT).unwrap();
}

#[cfg(feature = "spirv-reflection")]
lazy_static::lazy_static! {
    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}

const QUADS: u32 = 2_000_000;
const DIVIDE: u32 = 1;
const PER_CALL: u32 = QUADS / DIVIDE;

#[derive(Debug, Default)]
struct QuadsRenderPipelineDesc;

#[derive(Debug)]
struct QuadsRenderPipeline<B: hal::Backend> {
    indirect: Escape<Buffer<B>>,
    vertices: Escape<Buffer<B>>,
    descriptor_set: Escape<DescriptorSet<B>>,
}

impl<B, T> SimpleGraphicsPipelineDesc<B, T> for QuadsRenderPipelineDesc
where
    B: hal::Backend,
    T: ?Sized,
{
    type Pipeline = QuadsRenderPipeline<B>;

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
        return vec![Color::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)];
    }

    fn layout(&self) -> Layout {
        #[cfg(feature = "spirv-reflection")]
        return SHADER_REFLECTION.layout().unwrap();

        #[cfg(not(feature = "spirv-reflection"))]
        return Layout {
            sets: vec![rendy::graph::render::SetLayout {
                bindings: vec![hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: hal::pso::DescriptorType::StorageBuffer,
                    count: 1,
                    stage_flags: hal::pso::ShaderStageFlags::VERTEX,
                    immutable_samplers: false,
                }],
            }],
            push_constants: Vec::new(),
        };
    }

    fn buffers(&self) -> Vec<BufferAccess> {
        vec![BufferAccess {
            access: hal::buffer::Access::SHADER_READ,
            stages: hal::pso::PipelineStage::VERTEX_SHADER,
            usage: hal::buffer::Usage::STORAGE,
        }]
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<QuadsRenderPipeline<B>, gfx_hal::pso::CreationError> {
        assert_eq!(buffers.len(), 1);
        assert!(images.is_empty());

        let posvelbuff = ctx.get_buffer(buffers[0].id).unwrap();

        let mut indirect = factory
            .create_buffer(
                BufferInfo {
                    size: std::mem::size_of::<DrawCommand>() as u64 * DIVIDE as u64,
                    usage: hal::buffer::Usage::INDIRECT,
                },
                Dynamic,
            )
            .unwrap();

        unsafe {
            factory
                .upload_visible_buffer(
                    &mut indirect,
                    0,
                    &(0..DIVIDE)
                        .map(|index| DrawCommand {
                            vertex_count: 6,
                            instance_count: PER_CALL,
                            first_vertex: 0,
                            first_instance: index * PER_CALL,
                        })
                        .collect::<Vec<_>>(),
                )
                .unwrap();
        }

        let mut vertices = factory
            .create_buffer(
                BufferInfo {
                    size: std::mem::size_of::<Color>() as u64 * 6,
                    usage: hal::buffer::Usage::VERTEX,
                },
                Dynamic,
            )
            .unwrap();

        unsafe {
            factory
                .upload_visible_buffer(
                    &mut vertices,
                    0,
                    &[
                        Color({
                            let (r, g, b) = palette::Srgb::from(palette::Hsv::new(240.0, 1.0, 1.0))
                                .into_components();
                            [r, g, b, 1.0]
                        }),
                        Color({
                            let (r, g, b) = palette::Srgb::from(palette::Hsv::new(330.0, 1.0, 1.0))
                                .into_components();
                            [r, g, b, 1.0]
                        }),
                        Color({
                            let (r, g, b) = palette::Srgb::from(palette::Hsv::new(60.0, 1.0, 1.0))
                                .into_components();
                            [r, g, b, 1.0]
                        }),
                        Color({
                            let (r, g, b) = palette::Srgb::from(palette::Hsv::new(240.0, 1.0, 1.0))
                                .into_components();
                            [r, g, b, 1.0]
                        }),
                        Color({
                            let (r, g, b) = palette::Srgb::from(palette::Hsv::new(60.0, 1.0, 1.0))
                                .into_components();
                            [r, g, b, 1.0]
                        }),
                        Color({
                            let (r, g, b) = palette::Srgb::from(palette::Hsv::new(150.0, 1.0, 1.0))
                                .into_components();
                            [r, g, b, 1.0]
                        }),
                    ],
                )
                .unwrap();
        }

        assert_eq!(set_layouts.len(), 1);

        let descriptor_set = factory
            .create_descriptor_set(set_layouts[0].clone())
            .unwrap();

        unsafe {
            factory
                .device()
                .write_descriptor_sets(std::iter::once(hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: std::iter::once(hal::pso::Descriptor::Buffer(
                        posvelbuff.raw(),
                        Some(0)..Some(posvelbuff.size() as u64),
                    )),
                }))
        }

        Ok(QuadsRenderPipeline {
            indirect,
            vertices,
            descriptor_set,
        })
    }
}

impl<B, T> SimpleGraphicsPipeline<B, T> for QuadsRenderPipeline<B>
where
    B: hal::Backend,
    T: ?Sized,
{
    type Desc = QuadsRenderPipelineDesc;

    fn prepare(
        &mut self,
        _factory: &Factory<B>,
        _queue: QueueId,
        _sets: &[Handle<DescriptorSetLayout<B>>],
        _index: usize,
        _aux: &T,
    ) -> PrepareResult {
        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        _index: usize,
        _aux: &T,
    ) {
        unsafe {
            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                std::iter::once(self.descriptor_set.raw()),
                std::iter::empty::<u32>(),
            );
            encoder.bind_vertex_buffers(0, std::iter::once((self.vertices.raw(), 0)));
            encoder.draw_indirect(
                self.indirect.raw(),
                0,
                DIVIDE,
                std::mem::size_of::<DrawCommand>() as u32,
            );
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &T) {}
}

#[derive(Debug)]
struct GravBounce<B: hal::Backend> {
    set_layout: Handle<DescriptorSetLayout<B>>,
    pipeline_layout: B::PipelineLayout,
    pipeline: B::ComputePipeline,

    descriptor_set: Escape<DescriptorSet<B>>,

    command_pool: CommandPool<B, Compute>,
    command_buffer:
        CommandBuffer<B, Compute, PendingState<ExecutableState<MultiShot<SimultaneousUse>>>>,
    submit: Submit<B, SimultaneousUse>,
}

impl<'a, B> NodeSubmittable<'a, B> for GravBounce<B>
where
    B: hal::Backend,
{
    type Submittable = &'a Submit<B, SimultaneousUse>;
    type Submittables = &'a [Submit<B, SimultaneousUse>];
}

impl<B, T> Node<B, T> for GravBounce<B>
where
    B: hal::Backend,
    T: ?Sized,
{
    type Capability = Compute;

    type Desc = GravBounceDesc;

    fn run<'a>(
        &'a mut self,
        _ctx: &GraphContext<B>,
        _factory: &Factory<B>,
        _aux: &T,
        _frames: &'a Frames<B>,
    ) -> &'a [Submit<B, SimultaneousUse>] {
        std::slice::from_ref(&self.submit)
    }

    unsafe fn dispose(mut self, factory: &mut Factory<B>, _aux: &T) {
        drop(self.submit);
        self.command_pool
            .free_buffers(Some(self.command_buffer.mark_complete()));
        factory.destroy_command_pool(self.command_pool);
        factory.destroy_compute_pipeline(self.pipeline);
        factory.destroy_pipeline_layout(self.pipeline_layout);
    }
}

#[derive(Debug, Default)]
struct GravBounceDesc;

impl<B, T> NodeDesc<B, T> for GravBounceDesc
where
    B: hal::Backend,
    T: ?Sized,
{
    type Node = GravBounce<B>;

    fn buffers(&self) -> Vec<BufferAccess> {
        vec![BufferAccess {
            access: hal::buffer::Access::SHADER_READ | hal::buffer::Access::SHADER_WRITE,
            stages: hal::pso::PipelineStage::COMPUTE_SHADER,
            usage: hal::buffer::Usage::STORAGE | hal::buffer::Usage::TRANSFER_DST,
        }]
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        queue: usize,
        _aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Self::Node, NodeBuildError> {
        assert!(images.is_empty());
        assert_eq!(buffers.len(), 1);

        let posvelbuff = ctx.get_buffer(buffers[0].id).unwrap();

        unsafe {
            factory.upload_buffer(
                posvelbuff,
                0,
                &POSVEL_DATA,
                None,
                BufferState {
                    queue: QueueId {
                        index: queue,
                        family: family.id(),
                    },
                    stage: hal::pso::PipelineStage::COMPUTE_SHADER,
                    access: hal::buffer::Access::SHADER_WRITE | hal::buffer::Access::SHADER_READ,
                },
            )
            .map_err(NodeBuildError::Upload)
        }?;

        log::trace!("Load shader module BOUNCE_COMPUTE");
        let module = unsafe { BOUNCE_COMPUTE.module(factory) }
            .map_err(gfx_hal::pso::CreationError::Shader)
            .map_err(NodeBuildError::Pipeline)?;

        let set_layout = Handle::from(factory
            .create_descriptor_set_layout(vec![
                hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: hal::pso::DescriptorType::StorageBuffer,
                    count: 1,
                    stage_flags: hal::pso::ShaderStageFlags::COMPUTE,
                    immutable_samplers: false,
                },
            ])
            .map_err(NodeBuildError::OutOfMemory)?
        );

        let pipeline_layout = unsafe {
            factory
                .device()
                .create_pipeline_layout(
                    std::iter::once(set_layout.raw()),
                    std::iter::empty::<(hal::pso::ShaderStageFlags, std::ops::Range<u32>)>(),
                )
                .map_err(NodeBuildError::OutOfMemory)?
        };

        let pipeline = unsafe {
            factory
                .device()
                .create_compute_pipeline(
                    &hal::pso::ComputePipelineDesc {
                        shader: hal::pso::EntryPoint {
                            entry: "main",
                            module: &module,
                            specialization: hal::pso::Specialization::default(),
                        },
                        layout: &pipeline_layout,
                        flags: hal::pso::PipelineCreationFlags::empty(),
                        parent: hal::pso::BasePipeline::None,
                    },
                    None,
                )
                .map_err(NodeBuildError::Pipeline)?
        };

        unsafe { factory.destroy_shader_module(module) };

        let descriptor_set = factory
            .create_descriptor_set(set_layout.clone())
            .map_err(NodeBuildError::OutOfMemory)?;

        unsafe {
            factory
                .device()
                .write_descriptor_sets(std::iter::once(hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: std::iter::once(hal::pso::Descriptor::Buffer(
                        posvelbuff.raw(),
                        Some(0)..Some(posvelbuff.size()),
                    )),
                }));
        }

        let mut command_pool = factory
            .create_command_pool(family)
            .map_err(NodeBuildError::OutOfMemory)?
            .with_capability::<Compute>()
            .expect("Graph builder must provide family with Compute capability");
        let initial = command_pool.allocate_buffers(1).remove(0);
        let mut recording = initial.begin(MultiShot(SimultaneousUse), ());
        let mut encoder = recording.encoder();
        encoder.bind_compute_pipeline(&pipeline);
        unsafe {
            encoder.bind_compute_descriptor_sets(
                &pipeline_layout,
                0,
                std::iter::once(descriptor_set.raw()),
                std::iter::empty::<u32>(),
            );

            {
                let (stages, barriers) = gfx_acquire_barriers(ctx, &*buffers, None);
                log::info!("Acquire {:?} : {:#?}", stages, barriers);
                encoder.pipeline_barrier(stages, hal::memory::Dependencies::empty(), barriers);
            }
            encoder.dispatch(QUADS, 1, 1);

            {
                let (stages, barriers) = gfx_release_barriers(ctx, &*buffers, None);
                log::info!("Release {:?} : {:#?}", stages, barriers);
                encoder.pipeline_barrier(stages, hal::memory::Dependencies::empty(), barriers);
            }
        }

        let (submit, command_buffer) = recording.finish().submit();

        Ok(GravBounce {
            set_layout,
            pipeline_layout,
            pipeline,
            descriptor_set,
            // buffer_view,
            command_pool,
            command_buffer,
            submit,
        })
    }
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn run(
    event_loop: EventLoop<()>,
    mut factory: Factory<Backend>,
    mut families: Families<Backend>,
    window: Window,
) {
    let mut graph = Some(build_graph(&mut factory, &mut families, &window));

    let started = std::time::Instant::now();

    let mut frame = 0u64;
    let mut elapsed = started.elapsed();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::Resized(_dims) => {
                    let started = std::time::Instant::now();
                    graph.take().unwrap().dispose(&mut factory, &());
                    println!("Graph disposed in: {:?}", started.elapsed());
                    graph = Some(build_graph(&mut factory, &mut families, &window));
                }
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

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("quads", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let (factory, families): (Factory<Backend>, _) = rendy::factory::init(config).unwrap();

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)
        .unwrap();

    run(event_loop, factory, families, window);
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn build_graph(
    factory: &mut Factory<Backend>,
    families: &mut Families<Backend>,
    window: &Window,
) -> Graph<Backend, ()> {
    let surface = factory.create_surface(window);

    let mut graph_builder = GraphBuilder::<Backend, ()>::new();

    let posvel = graph_builder.create_buffer(QUADS as u64 * std::mem::size_of::<[f32; 4]>() as u64);

    let size = window
        .inner_size()
        .to_physical(window.hidpi_factor());
    let window_kind = hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1);

    let color = graph_builder.create_image(
        window_kind,
        1,
        factory.get_surface_format(&surface),
        Some(hal::command::ClearValue {
            color: hal::command::ClearColor {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        }),
    );

    let depth = graph_builder.create_image(
        window_kind,
        1,
        hal::format::Format::D16Unorm,
        Some(hal::command::ClearValue {
            depth_stencil: hal::command::ClearDepthStencil {
                depth: 1.0,
                stencil: 0,
            },
        }),
    );

    let grav = graph_builder.add_node(GravBounceDesc.builder().with_buffer(posvel));

    let pass = graph_builder.add_node(
        QuadsRenderPipeline::builder()
            .with_buffer(posvel)
            .with_dependency(grav)
            .into_subpass()
            .with_color(color)
            .with_depth_stencil(depth)
            .into_pass(),
    );

    graph_builder.add_node(PresentNode::builder(&factory, surface, color).with_dependency(pass));

    let started = std::time::Instant::now();
    let graph = graph_builder.build(factory, families, &()).unwrap();
    println!("Graph built in: {:?}", started.elapsed());
    graph
}

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
fn main() {
    panic!("Specify feature: { dx12, metal, vulkan }");
}
