//!
//! First non-trivial example simulates miriad of quads floating in gravity field and bouce of window borders.
//! It uses compute node to move quads and simple render pass to draw them.
//! 

#![cfg_attr(not(any(feature = "dx12", feature = "metal", feature = "vulkan")), allow(unused))]

use rendy::{
    command::{Compute, RenderPassInlineEncoder, Submit, CommandPool, CommandBuffer, PendingState, ExecutableState, MultiShot, SimultaneousUse, DrawCommand, FamilyId},
    factory::{Config, Factory},
    frame::{Frames},
    graph::{Graph, GraphBuilder, render::{RenderPass, Layout, SetLayout, PrepareResult}, present::PresentNode, NodeBuffer, NodeImage, BufferAccess, Node, NodeDesc, NodeSubmittable, gfx_acquire_barriers, gfx_release_barriers},
    memory::MemoryUsageValue,
    mesh::{AsVertex, Color},
    shader::{Shader, StaticShaderInfo, ShaderKind, SourceLanguage},
    resource::buffer::Buffer,
    hal::{Device, pso::DescriptorPool},
};

use winit::{
    EventsLoop, WindowBuilder,
};

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

lazy_static::lazy_static! {
    static ref RENDER_VERTEX: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/render.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );

    static ref RENDER_FRAGMENT: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/render.frag"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    );

    static ref BOUNCE_COMPUTE: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/quads/bounce.comp"),
        ShaderKind::Compute,
        SourceLanguage::GLSL,
        "main",
    );
}

const QUADS: u32 = 2_000_000;
const DIVIDE: u32 = 580;
const PER_CALL: u32 = QUADS / DIVIDE;

#[derive(Debug)]
struct QuadsRenderPass<B: gfx_hal::Backend> {
    indirect: Buffer<B>,
    vertices: Buffer<B>,

    descriptor_pool: B::DescriptorPool,
    descriptor_set: B::DescriptorSet,
    // buffer_view: B::BufferView,
}

impl<B, T> RenderPass<B, T> for QuadsRenderPass<B>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    fn name() -> &'static str {
        "Quads"
    }

    fn vertices() -> Vec<(
        Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>,
        gfx_hal::pso::ElemStride,
        gfx_hal::pso::InstanceRate,
    )> {
        vec![Color::VERTEX.gfx_vertex_input_desc(0)]
    }

    fn depth() -> bool {
        true
    }

    fn load_shader_sets<'a>(
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        _aux: &mut T,
    ) -> Vec<gfx_hal::pso::GraphicsShaderSet<'a, B>> {
        storage.clear();

        log::trace!("Load shader module '{:#?}'", *RENDER_VERTEX);
        storage.push(RENDER_VERTEX.module(factory).unwrap());

        log::trace!("Load shader module '{:#?}'", *RENDER_FRAGMENT);
        storage.push(RENDER_FRAGMENT.module(factory).unwrap());

        vec![gfx_hal::pso::GraphicsShaderSet {
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
        }]
    }

    fn buffers() -> Vec<BufferAccess> {
        vec![BufferAccess {
            access: gfx_hal::buffer::Access::SHADER_READ,
            stages: gfx_hal::pso::PipelineStage::VERTEX_SHADER,
            usage: gfx_hal::buffer::Usage::STORAGE,
        }]
    }

    fn layouts() -> Vec<Layout> {
        vec![Layout {
            sets: vec![SetLayout {
                bindings: vec![gfx_hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: gfx_hal::pso::DescriptorType::StorageBuffer,
                    count: 1,
                    stage_flags: gfx_hal::pso::ShaderStageFlags::VERTEX,
                    immutable_samplers: false,
                }]
            }],
            push_constants: Vec::new(),
        }]
    }

    fn build<'a>(
        factory: &mut Factory<B>,
        _aux: &mut T,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
        sets: &[impl AsRef<[B::DescriptorSetLayout]>],
    ) -> Self {
        assert_eq!(buffers.len(), 1);
        assert!(images.is_empty());

        let ref mut posvelbuff = buffers[0].buffer;

        let mut indirect = factory.create_buffer(512, std::mem::size_of::<DrawCommand>() as u64 * DIVIDE as u64, (gfx_hal::buffer::Usage::INDIRECT, MemoryUsageValue::Dynamic))
            .unwrap();

        unsafe {
            factory.upload_visible_buffer(&mut indirect, 0, &(0..DIVIDE).map(|index| DrawCommand {
                vertex_count: 6,
                instance_count: QUADS / DIVIDE,
                first_vertex: 0,
                first_instance: index * (QUADS / DIVIDE),
            }).collect::<Vec<_>>()).unwrap();
        }

        let mut vertices = factory.create_buffer(512, std::mem::size_of::<Color>() as u64 * 6, (gfx_hal::buffer::Usage::INDIRECT, MemoryUsageValue::Dynamic))
            .unwrap();

        unsafe {
            factory.upload_visible_buffer(&mut vertices, 0, &[
                Color({ let (r, g, b) = palette::Srgb::from(palette::Hsv::new(0.0, 1.0, 1.0)).into_components(); [r, g, b, 1.0] }),
                Color({ let (r, g, b) = palette::Srgb::from(palette::Hsv::new(90.0, 1.0, 1.0)).into_components(); [r, g, b, 1.0] }),
                Color({ let (r, g, b) = palette::Srgb::from(palette::Hsv::new(180.0, 1.0, 1.0)).into_components(); [r, g, b, 1.0] }),
                Color({ let (r, g, b) = palette::Srgb::from(palette::Hsv::new(0.0, 1.0, 1.0)).into_components(); [r, g, b, 1.0] }),
                Color({ let (r, g, b) = palette::Srgb::from(palette::Hsv::new(180.0, 1.0, 1.0)).into_components(); [r, g, b, 1.0] }),
                Color({ let (r, g, b) = palette::Srgb::from(palette::Hsv::new(270.0, 1.0, 1.0)).into_components(); [r, g, b, 1.0] }),
            ]).unwrap();

            let mut rng = rand::thread_rng();
            let uniform = rand::distributions::Uniform::new(0.0, 1.0);

            #[repr(C)]
            #[derive(Copy, Clone)]
            struct PosVel { pos: [f32; 2], vel: [f32; 2], }
            factory.upload_visible_buffer(posvelbuff, 0, &(0 .. QUADS).map(|_index| PosVel {
                pos: [rand::Rng::sample(&mut rng, uniform), rand::Rng::sample(&mut rng, uniform)],
                vel: [rand::Rng::sample(&mut rng, uniform), rand::Rng::sample(&mut rng, uniform)],
            }).collect::<Vec<PosVel>>()).unwrap();
        }

        assert_eq!(sets.len(), 1);
        let set_layouts = sets[0].as_ref();
        assert_eq!(set_layouts.len(), 1);

        let mut descriptor_pool = unsafe { gfx_hal::Device::create_descriptor_pool(
            factory.device(),
            1,
            std::iter::once(gfx_hal::pso::DescriptorRangeDesc {
                ty: gfx_hal::pso::DescriptorType::StorageBuffer,
                count: 1,
            }),
        ) }.unwrap();

        let descriptor_set = unsafe { gfx_hal::pso::DescriptorPool::allocate_set(
            &mut descriptor_pool,
            &set_layouts[0],
        ) }.unwrap();

        unsafe { gfx_hal::Device::write_descriptor_sets(
            factory.device(),
            std::iter::once(gfx_hal::pso::DescriptorSetWrite {
                set: &descriptor_set,
                binding: 0,
                array_offset: 0,
                descriptors: std::iter::once(gfx_hal::pso::Descriptor::Buffer(posvelbuff.raw(), Some(0) .. Some(posvelbuff.size() as u64))),
            }),
        ) }

        QuadsRenderPass {
            indirect,
            vertices,

            // buffer_view,
            descriptor_pool,
            descriptor_set,
        }
    }

    fn prepare(&mut self, _factory: &mut Factory<B>, _sets: &[impl AsRef<[B::DescriptorSetLayout]>], _index: usize, _aux: &T) -> PrepareResult {
        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layouts: &[B::PipelineLayout],
        pipelines: &[B::GraphicsPipeline],
        mut encoder: RenderPassInlineEncoder<'_, B>,
        _index: usize,
        _aux: &T,
    ) {
        encoder.bind_graphics_pipeline(&pipelines[0]);
        encoder.bind_graphics_descriptor_sets(
            &layouts[0],
            0,
            std::iter::once(&self.descriptor_set),
            std::iter::empty::<u32>(),
        );
        // encoder.bind_vertex_buffers(0, std::iter::once((self.vertices.raw(), 0)));
        // encoder.draw_indirect(self.indirect.raw(), 0, DIVIDE, std::mem::size_of::<DrawCommand>() as u32);

        for index in 0 .. DIVIDE {
            encoder.bind_vertex_buffers(0, std::iter::once((self.vertices.raw(), 0)));
            encoder.draw(0..6, index * PER_CALL .. (index + 1) * PER_CALL);
        }

    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &mut T) {
        
    }
}

#[derive(Debug)]
struct GravBounce<B: gfx_hal::Backend> {
    set_layout: B::DescriptorSetLayout,
    pipeline_layout: B::PipelineLayout,
    pipeline: B::ComputePipeline,

    descriptor_pool: B::DescriptorPool,
    descriptor_set: B::DescriptorSet,
    // buffer_view: B::BufferView,

    command_pool: CommandPool<B, Compute>,
    command_buffer: CommandBuffer<B, Compute, PendingState<ExecutableState<MultiShot<SimultaneousUse>>>>,
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
        _factory: &mut Factory<B>,
        _aux: &mut T,
        _frames: &'a Frames<B>,
    ) -> &'a [Submit<B, SimultaneousUse>] {
        std::slice::from_ref(&self.submit)
    }

    unsafe fn dispose(mut self, factory: &mut Factory<B>, _aux: &mut T) {
        drop(self.submit);
        self.command_pool.free_buffers(Some(self.command_buffer.mark_complete()));
        factory.destroy_command_pool(self.command_pool);
        self.descriptor_pool.free_sets(Some(self.descriptor_set));
        factory.destroy_descriptor_pool(self.descriptor_pool);
        factory.destroy_compute_pipeline(self.pipeline);
        factory.destroy_pipeline_layout(self.pipeline_layout);
        factory.destroy_descriptor_set_layout(self.set_layout);
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
            usage: gfx_hal::buffer::Usage::STORAGE,
        }]
    }

    fn build<'a>(
        &self,
        factory: &mut Factory<B>,
        _aux: &mut T,
        family: FamilyId,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
    ) -> Result<Self::Node, failure::Error> {
        assert!(images.is_empty());
        assert_eq!(buffers.len(), 1);

        let ref mut posvelbuff = buffers[0].buffer;

        log::trace!("Load shader module '{:#?}'", *BOUNCE_COMPUTE);
        let module = BOUNCE_COMPUTE.module(factory)?;

        let set_layout = unsafe { gfx_hal::Device::create_descriptor_set_layout(
            factory.device(),
            std::iter::once(gfx_hal::pso::DescriptorSetLayoutBinding {
                binding: 0,
                ty: gfx_hal::pso::DescriptorType::StorageBuffer,
                count: 1,
                stage_flags: gfx_hal::pso::ShaderStageFlags::COMPUTE,
                immutable_samplers: false,
            }),
            std::iter::empty::<B::Sampler>(),
        ) }?;

        let pipeline_layout = unsafe { gfx_hal::Device::create_pipeline_layout(
            factory.device(),
            std::iter::once(&set_layout),
            std::iter::empty::<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>(),
        ) }?;

        let pipeline = unsafe { gfx_hal::Device::create_compute_pipeline(
            factory.device(),
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
        ) }?;

        let (descriptor_pool, descriptor_set/*, buffer_view*/) = unsafe {
            let mut descriptor_pool = gfx_hal::Device::create_descriptor_pool(
                factory.device(),
                1,
                std::iter::once(gfx_hal::pso::DescriptorRangeDesc {
                    ty: gfx_hal::pso::DescriptorType::StorageBuffer,
                    count: 1,
                }),
            )?;

            let descriptor_set = gfx_hal::pso::DescriptorPool::allocate_set(
                &mut descriptor_pool,
                &set_layout
            )?;

            // let buffer_view = gfx_hal::Device::create_buffer_view(
            //     factory.device(),
            //     posvelbuff.raw(),
            //     Some(gfx_hal::format::Format::Rgba32Float),
            //     0 .. posvelbuff.size(),
            // )?;

            gfx_hal::Device::write_descriptor_sets(
                factory.device(),
                std::iter::once(gfx_hal::pso::DescriptorSetWrite {
                    set: &descriptor_set,
                    binding: 0,
                    array_offset: 0,
                    descriptors: std::iter::once(gfx_hal::pso::Descriptor::Buffer(posvelbuff.raw(), Some(0) .. Some(posvelbuff.size()))),
                }),
            );

            (descriptor_pool, descriptor_set/*, buffer_view*/)
        };

        let mut command_pool = factory.create_command_pool(family)?
            .with_capability::<Compute>()
            .expect("Graph builder must provide family with Compute capability");
        let initial = command_pool.allocate_buffers(1).remove(0);
        let mut recording = initial.begin(MultiShot(SimultaneousUse), ());
        let mut encoder = recording.encoder();
        encoder.bind_compute_pipeline(&pipeline);
        encoder.bind_compute_descriptor_sets(
            &pipeline_layout,
            0,
            std::iter::once(&descriptor_set),
            std::iter::empty::<u32>(),
        );

        {
            let (stages, barriers) = gfx_acquire_barriers(&*buffers, None);
            log::info!("Acquire {:?} : {:#?}", stages, barriers);
            encoder.pipeline_barrier(
                stages,
                gfx_hal::memory::Dependencies::empty(),
                barriers,
            );
        }
        encoder.dispatch(QUADS, 1, 1);

        {
            let (stages, barriers) = gfx_release_barriers(&*buffers, None);
            log::info!("Release {:?} : {:#?}", stages, barriers);
            encoder.pipeline_barrier(
                stages,
                gfx_hal::memory::Dependencies::empty(),
                barriers,
            );
        }

        let (submit, command_buffer) = recording.finish().submit();

        Ok(GravBounce {
            set_layout,
            pipeline_layout,
            pipeline,
            descriptor_pool,
            descriptor_set,
            // buffer_view,
            command_pool,
            command_buffer,
            submit,
        })
    }
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn run(event_loop: &mut EventsLoop, factory: &mut Factory<Backend>, mut graph: Graph<Backend, ()>) -> Result<(), failure::Error> {

    let started = std::time::Instant::now();

    let mut frames = 0u64 ..;
    let mut elapsed = started.elapsed();

    for _ in &mut frames {
        event_loop.poll_events(|_| ());
        graph.run(factory, &mut ());

        elapsed = started.elapsed();
        if elapsed >= std::time::Duration::new(5, 0) {
            break;
        }
    }

    let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

    log::info!("Elapsed: {:?}. Frames: {}. FPS: {}", elapsed, frames.start, frames.start * 1_000_000_000 / elapsed_ns);

    graph.dispose(factory, &mut ());
    Ok(())
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("quads", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let mut factory: Factory<Backend> = Factory::new(config).unwrap();

    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop).unwrap();

    event_loop.poll_events(|_| ());

    let surface = factory.create_surface(window.into());

    let mut graph_builder = GraphBuilder::<Backend, ()>::new();

    let posvel = graph_builder.create_buffer(
        QUADS as u64 * std::mem::size_of::<[f32; 4]>() as u64,
        MemoryUsageValue::Dynamic,
    );

    let color = graph_builder.create_image(
        surface.kind(),
        1,
        gfx_hal::format::Format::Rgba8Unorm,
        MemoryUsageValue::Data,
        Some(gfx_hal::command::ClearValue::Color([1.0, 1.0, 1.0, 1.0].into())),
    );

    let depth = graph_builder.create_image(
        surface.kind(),
        1,
        gfx_hal::format::Format::D16Unorm,
        MemoryUsageValue::Data,
        Some(gfx_hal::command::ClearValue::DepthStencil(gfx_hal::command::ClearDepthStencil(1.0, 0))),
    );

    let grav = graph_builder.add_node(
        GravBounce::builder()
            .with_buffer(posvel)
    );

    let pass = graph_builder.add_node(
        QuadsRenderPass::builder()
            .with_image(color)
            .with_image(depth)
            .with_buffer(posvel)
            .with_dependency(grav)
    );

    graph_builder.add_node(
        PresentNode::builder(surface)
            .with_image(color)
            .with_dependency(pass)
    );

    let graph = graph_builder.build(&mut factory, &mut ()).unwrap();

    run(&mut event_loop, &mut factory, graph).unwrap();
}

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
fn main() {
    panic!("Specify feature: { dx12, metal, vulkan }");
}
