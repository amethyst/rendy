//!
//! A simple sprite example.
//! This examples shows how to render a sprite on a white background.
//!

#![cfg_attr(
    not(any(feature = "dx12", feature = "metal", feature = "vulkan")),
    allow(unused)
)]

use rendy::{
    command::{Families, QueueId, RenderPassEncoder},
    factory::{Config, Factory, ImageState},
    graph::{
        present::PresentNode, render::*, Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage,
    },
    hal::{self, device::Device as _},
    memory::Dynamic,
    mesh::PosTex,
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{ShaderKind, SourceLanguage, SourceShaderInfo, SpirvShader},
    texture::{image::ImageTextureConfig, Texture},
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

use std::{fs::File, io::BufReader};

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sprite/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sprite/shader.frag").into(),
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
struct SpriteGraphicsPipelineDesc;

#[derive(Debug)]
struct SpriteGraphicsPipeline<B: hal::Backend> {
    texture: Texture<B>,
    vbuf: Escape<Buffer<B>>,
    descriptor_set: Escape<DescriptorSet<B>>,
}

impl<B, T> SimpleGraphicsPipelineDesc<B, T> for SpriteGraphicsPipelineDesc
where
    B: hal::Backend,
    T: ?Sized,
{
    type Pipeline = SpriteGraphicsPipeline<B>;

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
        return vec![PosTex::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)];
    }

    fn layout(&self) -> Layout {
        #[cfg(feature = "spirv-reflection")]
        return SHADER_REFLECTION.layout().unwrap();

        #[cfg(not(feature = "spirv-reflection"))]
        return Layout {
            sets: vec![SetLayout {
                bindings: vec![
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 0,
                        ty: hal::pso::DescriptorType::SampledImage,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 1,
                        ty: hal::pso::DescriptorType::Sampler,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                ],
            }],
            push_constants: Vec::new(),
        };
    }

    fn build<'b>(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<SpriteGraphicsPipeline<B>, hal::pso::CreationError> {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert_eq!(set_layouts.len(), 1);

        // This is how we can load an image and create a new texture.
        let image_reader = BufReader::new(
            File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/examples/sprite/logo.png"
            ))
            .map_err(|e| {
                log::error!("Unable to open {}: {:?}", "/examples/sprite/logo.png", e);
                hal::pso::CreationError::Other
            })?,
        );

        let texture_builder = rendy::texture::image::load_from_image(
            image_reader,
            ImageTextureConfig {
                generate_mips: true,
                ..Default::default()
            },
        )
        .map_err(|e| {
            log::error!("Unable to load image: {:?}", e);
            hal::pso::CreationError::Other
        })?;

        let texture = texture_builder
            .build(
                ImageState {
                    queue,
                    stage: hal::pso::PipelineStage::FRAGMENT_SHADER,
                    access: hal::image::Access::SHADER_READ,
                    layout: hal::image::Layout::ShaderReadOnlyOptimal,
                },
                factory,
            )
            .unwrap();

        let descriptor_set = factory
            .create_descriptor_set(set_layouts[0].clone())
            .unwrap();

        unsafe {
            factory.device().write_descriptor_sets(vec![
                hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: vec![hal::pso::Descriptor::Image(
                        texture.view().raw(),
                        hal::image::Layout::ShaderReadOnlyOptimal,
                    )],
                },
                hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 1,
                    array_offset: 0,
                    descriptors: vec![hal::pso::Descriptor::Sampler(texture.sampler().raw())],
                },
            ]);
        }

        #[cfg(feature = "spirv-reflection")]
        let vbuf_size = SHADER_REFLECTION.attributes_range(..).unwrap().stride as u64 * 6;

        #[cfg(not(feature = "spirv-reflection"))]
        let vbuf_size = PosTex::vertex().stride as u64 * 6;

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
                        PosTex {
                            position: [-0.5, 0.33, 0.0].into(),
                            tex_coord: [0.0, 1.0].into(),
                        },
                        PosTex {
                            position: [0.5, 0.33, 0.0].into(),
                            tex_coord: [1.0, 1.0].into(),
                        },
                        PosTex {
                            position: [0.5, -0.33, 0.0].into(),
                            tex_coord: [1.0, 0.0].into(),
                        },
                        PosTex {
                            position: [-0.5, 0.33, 0.0].into(),
                            tex_coord: [0.0, 1.0].into(),
                        },
                        PosTex {
                            position: [0.5, -0.33, 0.0].into(),
                            tex_coord: [1.0, 0.0].into(),
                        },
                        PosTex {
                            position: [-0.5, -0.33, 0.0].into(),
                            tex_coord: [0.0, 0.0].into(),
                        },
                    ],
                )
                .unwrap();
        }

        Ok(SpriteGraphicsPipeline {
            texture,
            vbuf,
            descriptor_set,
        })
    }
}

impl<B, T> SimpleGraphicsPipeline<B, T> for SpriteGraphicsPipeline<B>
where
    B: hal::Backend,
    T: ?Sized,
{
    type Desc = SpriteGraphicsPipelineDesc;

    fn prepare(
        &mut self,
        _factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
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
            encoder.bind_vertex_buffers(0, Some((self.vbuf.raw(), 0)));
            encoder.draw(0..6, 0..1);
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &T) {}
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn run(
    event_loop: EventLoop<()>,
    mut factory: Factory<Backend>,
    mut families: Families<Backend>,
    graph: Graph<Backend, ()>,
) {
    let started = std::time::Instant::now();

    std::thread::spawn(move || {
        while started.elapsed() < std::time::Duration::new(30, 0) {
            std::thread::sleep(std::time::Duration::new(1, 0));
        }

        std::process::abort();
    });

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

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("sprite", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let (mut factory, mut families): (Factory<Backend>, _) = rendy::factory::init(config).unwrap();

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)
        .unwrap();

    let surface = factory.create_surface(&window).unwrap();

    let mut graph_builder = GraphBuilder::<Backend, ()>::new();

    let size = window.inner_size().to_physical(window.hidpi_factor());

    let color = graph_builder.create_image(
        hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1),
        1,
        factory.get_surface_format(&surface),
        Some(hal::command::ClearValue {
            color: hal::command::ClearColor {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        }),
    );

    let pass = graph_builder.add_node(
        SpriteGraphicsPipeline::builder()
            .into_subpass()
            .with_color(color)
            .into_pass(),
    );

    graph_builder.add_node(PresentNode::builder(&factory, surface, color).with_dependency(pass));

    let graph = graph_builder
        .build(&mut factory, &mut families, &())
        .unwrap();

    run(event_loop, factory, families, graph);
}

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
fn main() {
    panic!("Specify feature: { dx12, metal, vulkan }");
}
