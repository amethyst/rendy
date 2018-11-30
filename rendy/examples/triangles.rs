
extern crate rendy;
extern crate winit;

use rendy::{
    command::{Graphics, Encoder, EncoderCommon, RenderPassEncoder},
    factory::{Config, Factory},
    frame::{cirque::CirqueRenderPassInlineEncoder},
    graph::{GraphBuilder, Graph, render::RenderPass, present::PresentNode},
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
}

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

fn run(event_loop: &mut EventsLoop, factory: &mut Factory<Backend>, mut renderer: impl Renderer<Backend, ()>) -> Result<(), failure::Error> {
    for _ in 0 .. 200 {
        event_loop.poll_events(|_| ());
        renderer.run(factory, &mut ());
        std::thread::sleep_ms(16);
    }

    renderer.dispose(factory, &mut ());
    Ok(())
}

fn main() {
    env_logger::Builder::from_default_env()
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

    let color = graph_builder.create_image(
        surface.kind(),
        1,
        gfx_hal::format::Format::Rgba8Unorm,
        MemoryUsageValue::Data,
        Some(gfx_hal::command::ClearValue::Color([1.0, 1.0, 1.0, 1.0].into())),
    );

    let pass = graph_builder.add_node(
        TrianglesRenderPass::builder()
            .with_image(color)
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