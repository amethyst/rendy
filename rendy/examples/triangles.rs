
extern crate rendy;
extern crate winit;

use rendy::{
    command::{Compute, Graphics, Encoder, EncoderCommon, RenderPassEncoder, Submit, CommandPool, CommandBuffer, PendingState, ExecutableState, MultiShot, SimultaneousUse, PrimaryLevel},
    factory::{Config, Factory},
    frame::{cirque::CirqueRenderPassInlineEncoder, Frames},
    graph::{GraphBuilder, Graph, render::RenderPass, present::PresentNode, NodeBuffer, NodeImage, BufferAccess, Node, NodeDesc, NodeSubmittable},
    memory::usage::MemoryUsageValue,
    mesh::{AsVertex, PosColor},
    shader::{Shader, StaticShaderInfo, ShaderKind, SourceLanguage},
    wsi::{Surface, Target},
    renderer::Renderer,
    resource::buffer::Buffer,
};

use winit::{
    EventsLoop, WindowBuilder,
};

#[cfg(feature = "empty")]
type Backend = rendy::empty::Backend;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

lazy_static::lazy_static! {
    static ref vertex: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangles.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );

    static ref fragment: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangles.frag"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    );

    static ref compute: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangles.comp"),
        ShaderKind::Compute,
        SourceLanguage::GLSL,
        "main",
    );
}

const MAX_TRIANGLES: u64 = 1024 * 1024;

#[derive(Debug)]
struct TrianglesRenderPass<B: gfx_hal::Backend> {
    vertex: Option<Buffer<B>>,
}

impl<B, T> RenderPass<B, T> for TrianglesRenderPass<B>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    fn name() -> &'static str {
        "Triangles"
    }

    fn vertices() -> Vec<(
        Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>,
        gfx_hal::pso::ElemStride,
    )> {
        vec![PosColor::VERTEX.gfx_vertex_input_desc()]
    }

    fn load_shader_sets<'a>(
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        aux: &mut T,
    ) -> Vec<gfx_hal::pso::GraphicsShaderSet<'a, B>> {
        storage.clear();

        log::trace!("Load shader module '{:#?}'", *vertex);
        storage.push(vertex.module(factory).unwrap());

        log::trace!("Load shader module '{:#?}'", *fragment);
        storage.push(fragment.module(factory).unwrap());

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

    fn build(
        sampled: &[B::ImageView],
        storage: &[B::ImageView],
        _factory: &mut Factory<B>,
        _aux: &mut T,
    ) -> Self {
        assert!(sampled.is_empty());
        assert!(storage.is_empty());

        TrianglesRenderPass {
            vertex: None,
        }
    }

    fn prepare(&mut self, _sets: &[impl AsRef<[B::DescriptorSetLayout]>], factory: &mut Factory<B>, aux: &T) -> bool {
        if self.vertex.is_some() {
            return false;
        }

        let mut vbuf = factory.create_buffer(512, PosColor::VERTEX.stride as u64 * 3, (gfx_hal::buffer::Usage::VERTEX, MemoryUsageValue::Dynamic))
            .unwrap();

        unsafe {
            /// Fresh buffer.
            factory.upload_visible_buffer(&mut vbuf, 0, &[
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
            ]).unwrap();
        }

        self.vertex = Some(vbuf);

        true
    }

    fn draw(
        &mut self,
        _layouts: &[B::PipelineLayout],
        pipelines: &[B::GraphicsPipeline],
        encoder: &mut CirqueRenderPassInlineEncoder<'_, B>,
        _aux: &T,
    ) {
        let vbuf = self.vertex.as_ref().unwrap();
        encoder.bind_graphics_pipeline(&pipelines[0]);
        encoder.bind_vertex_buffers(0, Some((vbuf, 0)));
        encoder.draw(0..3, 0..1);
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
    buffer_view: B::BufferView,

    command_pool: CommandPool<B, Compute>,
    command_buffer: CommandBuffer<B, Compute, PendingState<ExecutableState<MultiShot<SimultaneousUse>>>>,
    submit: Submit<'static, B, SimultaneousUse>,
}

impl<'a, B> NodeSubmittable<'a, B> for GravBounce<B>
where
    B: gfx_hal::Backend,
{
    type Submittable = &'a Submit<'a, B, SimultaneousUse>;
    type Submittables = &'a [Submit<'a, B, SimultaneousUse>];
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
        factory: &mut Factory<B>,
        aux: &mut T,
        frames: &'a Frames<B>,
    ) -> &'a [Submit<'a, B, SimultaneousUse>] {
        std::slice::from_ref(&self.submit)
    }

    unsafe fn dispose(self, factory: &mut Factory<B>, aux: &mut T) {
        
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
            usage: gfx_hal::buffer::Usage::STORAGE_TEXEL,
        }]
    }

    fn build<'a>(
        &self,
        factory: &mut Factory<B>,
        aux: &mut T,
        family: gfx_hal::queue::QueueFamilyId,
        buffers: &[NodeBuffer<'a, B>],
        images: &[NodeImage<'a, B>],
    ) -> Result<Self::Node, failure::Error> {
        assert!(images.is_empty());
        assert_eq!(buffers.len(), 1);

        log::trace!("Load shader module '{:#?}'", *compute);
        let module = compute.module(factory)?;

        let set_layout = gfx_hal::Device::create_descriptor_set_layout(
            factory.device(),
            std::iter::once(gfx_hal::pso::DescriptorSetLayoutBinding {
                binding: 0,
                ty: gfx_hal::pso::DescriptorType::StorageTexelBuffer,
                count: 1,
                stage_flags: gfx_hal::pso::ShaderStageFlags::COMPUTE,
                immutable_samplers: false,
            }),
            std::iter::empty::<B::Sampler>(),
        )?;

        let pipeline_layout = gfx_hal::Device::create_pipeline_layout(
            factory.device(),
            std::iter::once(&set_layout),
            std::iter::empty::<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>(),
        )?;

        let pipeline = gfx_hal::Device::create_compute_pipeline(
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
        )?;

        let (descriptor_pool, descriptor_set, buffer_view) = unsafe {
            let mut descriptor_pool = gfx_hal::Device::create_descriptor_pool(
                factory.device(),
                1,
                std::iter::once(gfx_hal::pso::DescriptorRangeDesc {
                    ty: gfx_hal::pso::DescriptorType::StorageTexelBuffer,
                    count: 1,
                }),
            )?;

            let descriptor_set = gfx_hal::pso::DescriptorPool::allocate_set(
                &mut descriptor_pool,
                &set_layout
            )?;

            let buffer_view = gfx_hal::Device::create_buffer_view(
                factory.device(),
                buffers[0].buffer.raw(),
                Some(gfx_hal::format::Format::Rgba32Float),
                0 .. buffers[0].buffer.size(),
            )?;

            gfx_hal::Device::write_descriptor_sets(
                factory.device(),
                std::iter::once(gfx_hal::pso::DescriptorSetWrite {
                    set: &descriptor_set,
                    binding: 0,
                    array_offset: 0,
                    descriptors: std::iter::once(gfx_hal::pso::Descriptor::StorageTexelBuffer(&buffer_view)),
                }),
            );

            (descriptor_pool, descriptor_set, buffer_view)
        };

        let mut command_pool = factory.create_command_pool(family, ())?
            .with_capability::<Compute>()
            .expect("Graph builder must provide family with Compute capability");
        let command_buffer = command_pool.allocate_buffers(PrimaryLevel, 1).remove(0);
        let mut encoder = command_buffer.begin(MultiShot(SimultaneousUse), ());
        encoder.bind_compute_pipeline(&pipeline);
        // encoder.dispatch(0, 0, 0);
        let command_buffer = encoder.finish();
        let (submit, command_buffer) = command_buffer.submit();

        Ok(GravBounce {
            set_layout,
            pipeline_layout,
            pipeline,
            descriptor_pool,
            descriptor_set,
            buffer_view,
            command_pool,
            command_buffer,
            submit,
        })
    }
}

fn run(event_loop: &mut EventsLoop, factory: &mut Factory<Backend>, mut renderer: impl Renderer<Backend, ()>) -> Result<(), failure::Error> {

    let started = std::time::Instant::now();

    std::thread::spawn(move || {
        while started.elapsed() < std::time::Duration::new(30, 0) {
            std::thread::sleep(std::time::Duration::new(1, 0));
        }

        std::process::abort();
    });

    let mut frames = 0u64 ..;
    let mut elapsed = started.elapsed();

    for _ in &mut frames {
        event_loop.poll_events(|_| ());
        renderer.run(factory, &mut ());

        elapsed = started.elapsed();
        if elapsed >= std::time::Duration::new(5, 0) {
            break;
        }
    }

    let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

    log::info!("Elapsed: {:?}. Frames: {}. FPS: {}", elapsed, frames.start, frames.start * 1_000_000_000 / elapsed_ns);

    renderer.dispose(factory, &mut ());
    Ok(())
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("triangles", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let mut factory: Factory<Backend> = Factory::new(config).unwrap();

    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop).unwrap();

    event_loop.poll_events(|_| ());

    let surface = factory.create_surface(window);

    let mut graph_builder = GraphBuilder::<Backend, ()>::new();

    let posvel = graph_builder.create_buffer(
        MAX_TRIANGLES * std::mem::size_of::<[f32; 4]>() as u64,
        MemoryUsageValue::Dynamic,
    );

    let color = graph_builder.create_image(
        surface.kind(),
        1,
        gfx_hal::format::Format::Rgba8Unorm,
        MemoryUsageValue::Data,
        Some(gfx_hal::command::ClearValue::Color([1.0, 1.0, 1.0, 1.0].into())),
    );

    let grav = graph_builder.add_node(
        GravBounce::builder()
            .with_buffer(posvel)
    );

    let pass = graph_builder.add_node(
        TrianglesRenderPass::builder()
            .with_image(color)
            .with_dependency(grav)
    );

    graph_builder.add_node(
        PresentNode::builder(surface)
            .with_image(color)
            .with_dependency(pass)
    );

    let graph = graph_builder.build(&mut factory, &mut ()).unwrap();

    run(&mut event_loop, &mut factory, graph).unwrap();

    factory.dispose();
}