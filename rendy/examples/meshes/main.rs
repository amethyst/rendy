//!
//! The mighty triangle example.
//! This examples shows colord triangle on white background.
//! Nothing fancy. Just prove that `rendy` works.
//!

use {
    genmesh::generators::{IndexedPolygon, SharedVertex},
    rand::distributions::{Distribution, Uniform},
    rendy::{
        command::{DrawIndexedCommand, QueueId, RenderPassEncoder},
        factory::{Config, Factory},
        graph::{render::*, GraphBuilder, GraphContext, NodeBuffer, NodeImage},
        hal::{self, adapter::PhysicalDevice as _, device::Device as _},
        init::winit::{
            dpi::Size as DpiSize,
            event::{Event, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::WindowBuilder,
        },
        init::AnyWindowedRendy,
        memory::Dynamic,
        mesh::{Mesh, Model, PosColorNorm},
        resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
        shader::{ShaderKind, SourceLanguage, SourceShaderInfo, SpirvShader},
    },
    std::{cmp::min, mem::size_of, time},
};

#[cfg(feature = "spirv-reflection")]
use rendy::shader::SpirvReflection;

#[cfg(not(feature = "spirv-reflection"))]
use rendy::mesh::AsVertex;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/meshes/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/meshes/shader.frag").into(),
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

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
struct Light {
    pos: nalgebra::Vector3<f32>,
    pad: f32,
    intensity: f32,
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
struct UniformArgs {
    proj: nalgebra::Matrix4<f32>,
    view: nalgebra::Matrix4<f32>,
    lights_count: i32,
    pad: [i32; 3],
    lights: [Light; MAX_LIGHTS],
}

#[derive(Debug)]
struct Camera {
    view: nalgebra::Projective3<f32>,
    proj: nalgebra::Perspective3<f32>,
}

#[derive(Debug)]
struct Scene<B: hal::Backend> {
    camera: Camera,
    object_mesh: Option<Mesh<B>>,
    objects: Vec<nalgebra::Transform3<f32>>,
    lights: Vec<Light>,
}

const MAX_LIGHTS: usize = 32;
const MAX_OBJECTS: usize = 10_000;
const UNIFORM_SIZE: u64 = size_of::<UniformArgs>() as u64;
const MODELS_SIZE: u64 = size_of::<Model>() as u64 * MAX_OBJECTS as u64;
const INDIRECT_SIZE: u64 = size_of::<DrawIndexedCommand>() as u64;

fn iceil(value: u64, scale: u64) -> u64 {
    ((value - 1) / scale + 1) * scale
}

fn buffer_frame_size(align: u64) -> u64 {
    iceil(UNIFORM_SIZE + MODELS_SIZE + INDIRECT_SIZE, align)
}

fn uniform_offset(index: usize, align: u64) -> u64 {
    buffer_frame_size(align) * index as u64
}

fn models_offset(index: usize, align: u64) -> u64 {
    uniform_offset(index, align) + UNIFORM_SIZE
}

fn indirect_offset(index: usize, align: u64) -> u64 {
    models_offset(index, align) + MODELS_SIZE
}

#[derive(Debug, Default)]
struct MeshRenderPipelineDesc;

#[derive(Debug)]
struct MeshRenderPipeline<B: hal::Backend> {
    align: u64,
    buffer: Escape<Buffer<B>>,
    sets: Vec<Escape<DescriptorSet<B>>>,
}

impl<B> SimpleGraphicsPipelineDesc<B, Scene<B>> for MeshRenderPipelineDesc
where
    B: hal::Backend,
{
    type Pipeline = MeshRenderPipeline<B>;

    fn load_shader_set(
        &self,
        factory: &mut Factory<B>,
        _scene: &Scene<B>,
    ) -> rendy_shader::ShaderSet<B> {
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
        return vec![
            SHADER_REFLECTION
                .attributes(&["position", "color", "normal"])
                .unwrap()
                .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex),
            SHADER_REFLECTION
                .attributes_range(3..7)
                .unwrap()
                .gfx_vertex_input_desc(hal::pso::VertexInputRate::Instance(1)),
        ];

        #[cfg(not(feature = "spirv-reflection"))]
        return vec![
            PosColorNorm::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex),
            Model::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Instance(1)),
        ];
    }

    fn layout(&self) -> Layout {
        #[cfg(feature = "spirv-reflection")]
        return SHADER_REFLECTION.layout().unwrap();

        #[cfg(not(feature = "spirv-reflection"))]
        return Layout {
            sets: vec![SetLayout {
                bindings: vec![hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: hal::pso::DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: hal::pso::ShaderStageFlags::GRAPHICS,
                    immutable_samplers: false,
                }],
            }],
            push_constants: Vec::new(),
        };
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _scene: &Scene<B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<MeshRenderPipeline<B>, rendy_core::hal::pso::CreationError> {
        let frames = ctx.frames_in_flight as _;
        let align = factory
            .physical()
            .limits()
            .min_uniform_buffer_offset_alignment;

        let buffer = factory
            .create_buffer(
                BufferInfo {
                    size: buffer_frame_size(align) * frames as u64,
                    usage: hal::buffer::Usage::UNIFORM
                        | hal::buffer::Usage::INDIRECT
                        | hal::buffer::Usage::VERTEX,
                },
                Dynamic,
            )
            .unwrap();

        let mut sets = Vec::new();
        for index in 0..frames {
            unsafe {
                let set = factory
                    .create_descriptor_set(set_layouts[0].clone())
                    .unwrap();
                factory.write_descriptor_sets(Some(hal::pso::DescriptorSetWrite {
                    set: set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: Some(hal::pso::Descriptor::Buffer(
                        buffer.raw(),
                        hal::buffer::SubRange {
                            offset: uniform_offset(index, align),
                            size: Some(UNIFORM_SIZE),
                        },
                    )),
                }));
                sets.push(set);
            }
        }

        Ok(MeshRenderPipeline {
            align,
            buffer,
            sets,
        })
    }
}

impl<B> SimpleGraphicsPipeline<B, Scene<B>> for MeshRenderPipeline<B>
where
    B: hal::Backend,
{
    type Desc = MeshRenderPipelineDesc;

    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        index: usize,
        scene: &Scene<B>,
    ) -> PrepareResult {
        unsafe {
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    uniform_offset(index, self.align),
                    &[UniformArgs {
                        pad: [0, 0, 0],
                        proj: scene.camera.proj.to_homogeneous(),
                        view: scene.camera.view.inverse().to_homogeneous(),
                        lights_count: scene.lights.len() as i32,
                        lights: {
                            let mut array = [Light {
                                pad: 0.0,
                                pos: nalgebra::Vector3::new(0.0, 0.0, 0.0),
                                intensity: 0.0,
                            }; MAX_LIGHTS];
                            let count = min(scene.lights.len(), 32);
                            array[..count].copy_from_slice(&scene.lights[..count]);
                            array
                        },
                    }],
                )
                .unwrap()
        };

        unsafe {
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    indirect_offset(index, self.align),
                    &[DrawIndexedCommand {
                        index_count: scene.object_mesh.as_ref().unwrap().len(),
                        instance_count: scene.objects.len() as u32,
                        first_index: 0,
                        vertex_offset: 0,
                        first_instance: 0,
                    }],
                )
                .unwrap()
        };

        if !scene.objects.is_empty() {
            unsafe {
                factory
                    .upload_visible_buffer(
                        &mut self.buffer,
                        models_offset(index, self.align),
                        &scene.objects[..],
                    )
                    .unwrap()
            };
        }

        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        scene: &Scene<B>,
    ) {
        unsafe {
            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                Some(self.sets[index].raw()),
                std::iter::empty(),
            );

            #[cfg(feature = "spirv-reflection")]
            let vertex = [SHADER_REFLECTION
                .attributes(&["position", "color", "normal"])
                .unwrap()];

            #[cfg(not(feature = "spirv-reflection"))]
            let vertex = [PosColorNorm::vertex()];

            scene
                .object_mesh
                .as_ref()
                .unwrap()
                .bind(0, &vertex, &mut encoder)
                .unwrap();

            encoder.bind_vertex_buffers(
                1,
                std::iter::once((self.buffer.raw(), models_offset(index, self.align))),
            );
            encoder.draw_indexed_indirect(
                self.buffer.raw(),
                indirect_offset(index, self.align),
                1,
                INDIRECT_SIZE as u32,
            );
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _scene: &Scene<B>) {}
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("meshes", log::LevelFilter::Trace)
        .init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(DpiSize::Logical((960, 640).into()))
        .with_title("Rendy example");

    let config: Config = Default::default();
    let rendy = AnyWindowedRendy::init_auto(&config, window, &event_loop).unwrap();
    rendy::with_any_windowed_rendy!((rendy)
        use back; (mut factory, mut families, surface, window) => {

        let mut graph_builder = GraphBuilder::<_, Scene<_>>::new();

        let size = window.inner_size();
        let window_kind = hal::image::Kind::D2(size.width as u32, size.height as u32, 1, 1);
        let aspect = size.width / size.height;

        let depth = graph_builder.create_image(
            window_kind,
            1,
            hal::format::Format::D32Sfloat,
            Some(hal::command::ClearValue {
                depth_stencil: hal::command::ClearDepthStencil {
                    depth: 1.0,
                    stencil: 0,
                },
            }),
        );

        let pass = graph_builder.add_node(
            MeshRenderPipeline::builder()
                .into_subpass()
                .with_color_surface()
                .with_depth_stencil(depth)
                .into_pass()
                .with_surface(
                    surface,
                    hal::window::Extent2D {
                        width: size.width as _,
                        height: size.height as _,
                    },
                    Some(hal::command::ClearValue {
                        color: hal::command::ClearColor {
                            float32: [1.0, 1.0, 1.0, 1.0],
                        },
                    }),
                ),
        );

        let mut scene = Scene {
            camera: Camera {
                proj: nalgebra::Perspective3::new(aspect as f32, 3.1415 / 4.0, 1.0, 200.0),
                view: nalgebra::Projective3::identity() * nalgebra::Translation3::new(0.0, 0.0, 10.0),
            },
            object_mesh: None,
            objects: vec![],
            lights: vec![
                Light {
                    pad: 0.0,
                    pos: nalgebra::Vector3::new(0.0, 0.0, 0.0),
                    intensity: 10.0,
                },
                Light {
                    pad: 0.0,
                    pos: nalgebra::Vector3::new(0.0, 20.0, -20.0),
                    intensity: 140.0,
                },
                Light {
                    pad: 0.0,
                    pos: nalgebra::Vector3::new(-20.0, 0.0, -60.0),
                    intensity: 100.0,
                },
                Light {
                    pad: 0.0,
                    pos: nalgebra::Vector3::new(20.0, -30.0, -100.0),
                    intensity: 160.0,
                },
            ],
        };

        log::info!("{:#?}", scene);

        let graph = graph_builder
            .build(&mut factory, &mut families, &scene)
            .unwrap();

        let icosphere = genmesh::generators::IcoSphere::subdivide(4);
        let indices: Vec<_> = genmesh::Vertices::vertices(icosphere.indexed_polygon_iter())
            .map(|i| i as u32)
            .collect();
        let vertices: Vec<_> = icosphere
            .shared_vertex_iter()
            .map(|v| PosColorNorm {
                position: v.pos.into(),
                color: [
                    (v.pos.x + 1.0) / 2.0,
                    (v.pos.y + 1.0) / 2.0,
                    (v.pos.z + 1.0) / 2.0,
                    1.0,
                ]
                .into(),
                normal: v.normal.into(),
            })
            .collect();

        scene.object_mesh = Some(
            Mesh::<back::Backend>::builder()
                .with_indices(&indices[..])
                .with_vertices(&vertices[..])
                .build(graph.node_queue(pass), &factory)
                .unwrap(),
        );

        let started = time::Instant::now();

        let mut rng = rand::thread_rng();
        let rxy = Uniform::new(-1.0, 1.0);
        let rz = Uniform::new(0.0, 185.0);

        let mut fpss = Vec::new();
        let mut checkpoint = started;

        let mut frame = 0u64;
        let mut start = frame;
        let mut from = 0;
        let mut graph = Some(graph);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                Event::MainEventsCleared => {
                    factory.maintain(&mut families);
                    if let Some(ref mut graph) = graph {
                        graph.run(&mut factory, &mut families, &scene);
                        frame += 1;
                    }

                    if scene.objects.len() < MAX_OBJECTS {
                        scene.objects.push({
                            let z = rz.sample(&mut rng);
                            nalgebra::Transform3::identity()
                                * nalgebra::Translation3::new(
                                    rxy.sample(&mut rng) * (z / 2.0 + 4.0),
                                    rxy.sample(&mut rng) * (z / 2.0 + 4.0),
                                    -z,
                                )
                        })
                    } else {
                        *control_flow = ControlFlow::Exit
                    }

                    let elapsed = checkpoint.elapsed();
                    if elapsed >= std::time::Duration::new(5, 0) || *control_flow == ControlFlow::Exit {
                        let frames = frame - start;
                        let nanos = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;
                        fpss.push((frames * 1_000_000_000 / nanos, from..scene.objects.len()));
                        checkpoint += elapsed;
                        start = frame;
                        from = scene.objects.len();
                    }
                }
                _ => {}
            }

            if *control_flow == ControlFlow::Exit {
                log::info!("FPS: {:#?}", fpss);
                if let Some(graph) = graph.take() {
                    graph.dispose(&mut factory, &scene);
                }
                drop(scene.object_mesh.take());
            }
        });
    });
}
