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
        BufferAccess, Graph, GraphBuilder, GraphContext, Node, NodeBuffer, NodeDesc, NodeImage,
        NodeSubmittable,
    },
    hal::Device as _,
    memory::Dynamic,
    mesh::{Color},
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{Shader, ShaderKind, SourceLanguage, StaticShaderInfo},
};

#[cfg(feature = "spirv-reflection")]
use rendy::shader::SpirvReflectedShader as SpirvShader;

#[cfg(not(feature = "spirv-reflection"))]
use rendy::shader::SpirvShader as SpirvShader;

use winit::{EventsLoop, WindowBuilder};

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
    static ref RENDER_VERTEX: SpirvShader = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/render.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref RENDER_FRAGMENT: SpirvShader = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/render.frag"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref BOUNCE_COMPUTE: SpirvShader = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/bounce.comp"),
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
}

const QUADS: u32 = 2_000_000;
const DIVIDE: u32 = 1;
const PER_CALL: u32 = QUADS / DIVIDE;

#[derive(Debug, Default)]
struct QuadsRenderPipelineDesc;

#[derive(Debug)]
struct QuadsRenderPipeline<B: gfx_hal::Backend> {
    indirect: Escape<Buffer<B>>,
    vertices: Escape<Buffer<B>>,
    descriptor_set: Escape<DescriptorSet<B>>,
}

impl<B, T> SimpleGraphicsPipelineDesc<B, T> for QuadsRenderPipelineDesc
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    type Pipeline = QuadsRenderPipeline<B>;

    #[cfg(not(feature = "spirv-reflection"))]
    fn vertices(
        &self,
    ) -> Vec<(
        Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>,
        gfx_hal::pso::ElemStride,
        gfx_hal::pso::InstanceRate,
    )> {
        use rendy::mesh::AsVertex;
        vec![Color::VERTEX.gfx_vertex_input_desc(0)]
    }

    #[cfg(feature = "spirv-reflection")]
    fn vertices(
        &self,
    ) -> Vec<(
        Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>,
        gfx_hal::pso::ElemStride,
        gfx_hal::pso::InstanceRate,
    )> {
        use rendy::graph::reflect::ShaderLayoutGenerator;
        vec![RENDER_VERTEX.attributes(..).unwrap().gfx_vertex_input_desc(0)]
    }

    fn load_shader_set<'a>(
        &self,
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        _aux: &T,
    ) -> gfx_hal::pso::GraphicsShaderSet<'a, B> {
        storage.clear();

        log::trace!("Load shader module RENDER_VERTEX");
        storage.push(unsafe { RENDER_VERTEX.module(factory).unwrap() });

        log::trace!("Load shader module RENDER_FRAGMENT");
        storage.push(unsafe { RENDER_FRAGMENT.module(factory).unwrap() });

        gfx_hal::pso::GraphicsShaderSet {
            vertex: gfx_hal::pso::EntryPoint {
                entry: "main",
                module: &storage[0],
                specialization: gfx_hal::pso::Specialization::default(),
            },
            fragment: Some(gfx_hal::pso::EntryPoint {
                entry: "main",
                module: &storage[1],
                specialization: gfx_hal::pso::Specialization::default(),
            }),
            hull: None,
            domain: None,
            geometry: None,
        }
    }

    fn buffers(&self) -> Vec<BufferAccess> {
        vec![BufferAccess {
            access: gfx_hal::buffer::Access::SHADER_READ,
            stages: gfx_hal::pso::PipelineStage::VERTEX_SHADER,
            usage: gfx_hal::buffer::Usage::STORAGE,
        }]
    }

    #[cfg(not(feature = "spirv-reflection"))]
    fn layout(&self) -> Layout {
        use rendy::graph::render::SetLayout;

        Layout {
            sets: vec![SetLayout {
                bindings: vec![gfx_hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: gfx_hal::pso::DescriptorType::StorageBuffer,
                    count: 1,
                    stage_flags: gfx_hal::pso::ShaderStageFlags::VERTEX,
                    immutable_samplers: false,
                }],
            }],
            push_constants: Vec::new(),
        }
    }

    #[cfg(feature = "spirv-reflection")]
    fn layout(&self) -> Layout {
        use rendy::graph::reflect::SpirvLayoutMerger;
        vec![RENDER_VERTEX.clone(), RENDER_FRAGMENT.clone()].merge().unwrap()
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
    ) -> Result<QuadsRenderPipeline<B>, failure::Error> {
        assert_eq!(buffers.len(), 1);
        assert!(images.is_empty());

        let posvelbuff = ctx.get_buffer(buffers[0].id).unwrap();

        let mut indirect = factory
            .create_buffer(
                BufferInfo {
                    size: std::mem::size_of::<DrawCommand>() as u64 * DIVIDE as u64,
                    usage: gfx_hal::buffer::Usage::INDIRECT,
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
                    usage: gfx_hal::buffer::Usage::VERTEX,
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
            factory.device().write_descriptor_sets(std::iter::once(
                gfx_hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: std::iter::once(gfx_hal::pso::Descriptor::Buffer(
                        posvelbuff.raw(),
                        Some(0)..Some(posvelbuff.size() as u64),
                    )),
                },
            ))
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
    B: gfx_hal::Backend,
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

    fn dispose(self, _factory: &mut Factory<B>, _aux: &T) {}
}

#[derive(Debug)]
struct GravBounce<B: gfx_hal::Backend> {
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
    B: gfx_hal::Backend,
{
    type Submittable = &'a Submit<B, SimultaneousUse>;
    type Submittables = &'a [Submit<B, SimultaneousUse>];
}

impl<B, T> Node<B, T> for GravBounce<B>
where
    B: gfx_hal::Backend,
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
    B: gfx_hal::Backend,
    T: ?Sized,
{
    type Node = GravBounce<B>;

    fn buffers(&self) -> Vec<BufferAccess> {
        vec![BufferAccess {
            access: gfx_hal::buffer::Access::SHADER_READ | gfx_hal::buffer::Access::SHADER_WRITE,
            stages: gfx_hal::pso::PipelineStage::COMPUTE_SHADER,
            usage: gfx_hal::buffer::Usage::STORAGE | gfx_hal::buffer::Usage::TRANSFER_DST,
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
    ) -> Result<Self::Node, failure::Error> {
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
                    stage: gfx_hal::pso::PipelineStage::COMPUTE_SHADER,
                    access: gfx_hal::buffer::Access::SHADER_WRITE
                        | gfx_hal::buffer::Access::SHADER_READ,
                },
            )
        }?;

        log::trace!("Load shader module BOUNCE_COMPUTE");
        let module = unsafe { BOUNCE_COMPUTE.module(factory) }?;

        let set_layout = Handle::from(factory.create_descriptor_set_layout(vec![
            gfx_hal::pso::DescriptorSetLayoutBinding {
                binding: 0,
                ty: gfx_hal::pso::DescriptorType::StorageBuffer,
                count: 1,
                stage_flags: gfx_hal::pso::ShaderStageFlags::COMPUTE,
                immutable_samplers: false,
            },
        ])?);

        let pipeline_layout = unsafe {
            factory.device().create_pipeline_layout(
                std::iter::once(set_layout.raw()),
                std::iter::empty::<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>(),
            )
        }?;

        let pipeline = unsafe {
            factory.device().create_compute_pipeline(
                &gfx_hal::pso::ComputePipelineDesc {
                    shader: gfx_hal::pso::EntryPoint {
                        entry: "main",
                        module: &module,
                        specialization: gfx_hal::pso::Specialization::default(),
                    },
                    layout: &pipeline_layout,
                    flags: gfx_hal::pso::PipelineCreationFlags::empty(),
                    parent: gfx_hal::pso::BasePipeline::None,
                },
                None,
            )
        }?;

        unsafe { factory.destroy_shader_module(module) };

        let descriptor_set = factory.create_descriptor_set(set_layout.clone())?;

        unsafe {
            factory.device().write_descriptor_sets(std::iter::once(
                gfx_hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: std::iter::once(gfx_hal::pso::Descriptor::Buffer(
                        posvelbuff.raw(),
                        Some(0)..Some(posvelbuff.size()),
                    )),
                },
            ));
        }

        let mut command_pool = factory
            .create_command_pool(family)?
            .with_capability::<Compute>()
            .expect("Graph builder must provide family with Compute capability");
        let initial = command_pool.allocate_buffers(1).remove(0);
        let mut recording = initial.begin(MultiShot(SimultaneousUse), ());
        let mut encoder = recording.encoder();
        encoder.bind_compute_pipeline(&pipeline);
        encoder.bind_compute_descriptor_sets(
            &pipeline_layout,
            0,
            std::iter::once(descriptor_set.raw()),
            std::iter::empty::<u32>(),
        );

        {
            let (stages, barriers) = gfx_acquire_barriers(ctx, &*buffers, None);
            log::info!("Acquire {:?} : {:#?}", stages, barriers);
            encoder.pipeline_barrier(stages, gfx_hal::memory::Dependencies::empty(), barriers);
        }
        encoder.dispatch(QUADS, 1, 1);

        {
            let (stages, barriers) = gfx_release_barriers(ctx, &*buffers, None);
            log::info!("Release {:?} : {:#?}", stages, barriers);
            encoder.pipeline_barrier(stages, gfx_hal::memory::Dependencies::empty(), barriers);
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
    event_loop: &mut EventsLoop,
    factory: &mut Factory<Backend>,
    families: &mut Families<Backend>,
    window: std::sync::Arc<winit::Window>,
) -> Result<(), failure::Error> {
    let mut graph = build_graph(factory, families, window.clone());

    let started = std::time::Instant::now();

    let mut last_window_size = window.get_inner_size();
    let mut need_rebuild = false;

    let mut frames = 0u64..;
    let mut elapsed = started.elapsed();

    for _ in &mut frames {
        factory.maintain(families);
        event_loop.poll_events(|_| ());
        let new_window_size = window.get_inner_size();

        if last_window_size != new_window_size {
            need_rebuild = true;
        }

        if need_rebuild && last_window_size == new_window_size {
            need_rebuild = false;
            let started = std::time::Instant::now();
            graph.dispose(factory, &());
            println!("Graph disposed in: {:?}", started.elapsed());
            graph = build_graph(factory, families, window.clone());
        }

        last_window_size = new_window_size;

        graph.run(factory, families, &());

        elapsed = started.elapsed();
        if elapsed >= std::time::Duration::new(1, 0) {
            break;
        }
    }

    let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

    log::info!(
        "Elapsed: {:?}. Frames: {}. FPS: {}",
        elapsed,
        frames.start,
        frames.start * 1_000_000_000 / elapsed_ns
    );

    graph.dispose(factory, &mut ());
    Ok(())
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("quads", log::LevelFilter::Trace)
        .filter_module("rendy_graph", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let (mut factory, mut families): (Factory<Backend>, _) = rendy::factory::init(config).unwrap();

    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)
        .unwrap()
        .into();

    event_loop.poll_events(|_| ());

    run(&mut event_loop, &mut factory, &mut families, window).unwrap();
    log::debug!("Done");

    log::debug!("Drop families");
    drop(families);

    log::debug!("Drop factory");
    drop(factory);
}

fn build_graph(
    factory: &mut Factory<Backend>,
    families: &mut Families<Backend>,
    window: std::sync::Arc<winit::Window>,
) -> Graph<Backend, ()> {
    let surface = factory.create_surface(window.clone());

    let mut graph_builder = GraphBuilder::<Backend, ()>::new();

    let posvel = graph_builder.create_buffer(QUADS as u64 * std::mem::size_of::<[f32; 4]>() as u64);

    let color = graph_builder.create_image(
        surface.kind(),
        1,
        factory.get_surface_format(&surface),
        Some(gfx_hal::command::ClearValue::Color(
            [1.0, 1.0, 1.0, 1.0].into(),
        )),
    );

    let depth = graph_builder.create_image(
        surface.kind(),
        1,
        gfx_hal::format::Format::D16Unorm,
        Some(gfx_hal::command::ClearValue::DepthStencil(
            gfx_hal::command::ClearDepthStencil(1.0, 0),
        )),
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
