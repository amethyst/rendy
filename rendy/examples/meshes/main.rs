//!
//! The mighty triangle example.
//! This examples shows colord triangle on white background.
//! Nothing fancy. Just prove that `rendy` works.
//! 

#![cfg_attr(not(any(feature = "dx12", feature = "metal", feature = "vulkan")), allow(unused))]

use rendy::{
    command::{RenderPassEncoder, DrawIndexedCommand},
    factory::{Config, Factory},
    graph::{GraphBuilder, render::*, present::PresentNode, NodeBuffer, NodeImage},
    memory::MemoryUsageValue,
    mesh::{AsVertex, PosColorNorm, Mesh, Transform},
    shader::{Shader, StaticShaderInfo, ShaderKind, SourceLanguage},
    resource::buffer::Buffer,
    hal::{Device, pso::DescriptorPool},
};

use std::{time, mem::size_of, cmp::min};

use genmesh::generators::{IndexedPolygon, SharedVertex};

use rand::distributions::{Distribution, Uniform};

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
    static ref VERTEX: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/meshes/shader.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );

    static ref FRAGMENT: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/meshes/shader.frag"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    );
}

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
struct Light {
    pos: nalgebra::Vector3<f32>,
    pad: f32,
    intencity: f32,
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
struct Scene<B: gfx_hal::Backend> {
    camera: Camera,
    object_mesh: Option<Mesh<B>>,
    objects: Vec<nalgebra::Transform3<f32>>,
    lights: Vec<Light>,
}

const MAX_LIGHTS: usize = 32;
const MAX_OBJECTS: usize = 1024 * 8;

const UBERALIGN: u64 = 256;
const MAX_FRAMES: u64 = 5;
const UNIFORM_SIZE: u64 = size_of::<UniformArgs>() as u64;
const TRANSFORMS_SIZE: u64 = size_of::<Transform>() as u64 * MAX_OBJECTS as u64;
const INDIRECT_SIZE: u64 = size_of::<DrawIndexedCommand>() as u64;
const BUFFER_FRAME_SIZE: u64 = ((UNIFORM_SIZE + TRANSFORMS_SIZE + INDIRECT_SIZE - 1) / UBERALIGN + 1) * UBERALIGN;

const fn uniform_offset(index: usize) -> u64 {
    BUFFER_FRAME_SIZE * index as u64
}

const fn transforms_offset(index: usize) -> u64 {
    uniform_offset(index) + UNIFORM_SIZE
}

const fn indirect_offset(index: usize) -> u64 {
    transforms_offset(index) + TRANSFORMS_SIZE
}

#[derive(Debug)]
struct MeshRenderPipeline<B: gfx_hal::Backend> {
    descriptor_pool: B::DescriptorPool,
    buffer: Buffer<B>,
    sets: Vec<Option<B::DescriptorSet>>,
}

impl<B> SimpleGraphicsPipeline<B, Scene<B>> for MeshRenderPipeline<B>
where
    B: gfx_hal::Backend,
{
    fn name() -> &'static str {
        "Mesh"
    }

    fn layout() -> Layout {
        Layout {
            sets: vec![SetLayout {
                bindings: vec![gfx_hal::pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: gfx_hal::pso::DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: gfx_hal::pso::ShaderStageFlags::GRAPHICS,
                    immutable_samplers: false,
                }]
            }],
            push_constants: Vec::new(),
        }
    }

    fn vertices() -> Vec<(
        Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>,
        gfx_hal::pso::ElemStride,
        gfx_hal::pso::InstanceRate,
    )> {
        vec![
            PosColorNorm::VERTEX.gfx_vertex_input_desc(0),
            Transform::VERTEX.gfx_vertex_input_desc(1),
        ]
    }

    fn load_shader_set<'a>(
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        _aux: &mut Scene<B>,
    ) -> gfx_hal::pso::GraphicsShaderSet<'a, B> {
        storage.clear();

        log::trace!("Load shader module '{:#?}'", *VERTEX);
        storage.push(VERTEX.module(factory).unwrap());

        log::trace!("Load shader module '{:#?}'", *FRAGMENT);
        storage.push(FRAGMENT.module(factory).unwrap());

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

    fn build<'a>(
        factory: &mut Factory<B>,
        _aux: &mut Scene<B>,
        buffers: Vec<NodeBuffer<'a, B>>,
        images: Vec<NodeImage<'a, B>>,
        set_layouts: &[B::DescriptorSetLayout],
    ) -> Result<Self, failure::Error> {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert_eq!(set_layouts.len(), 1);
        
        let descriptor_pool = unsafe { factory.create_descriptor_pool(
            5,
            Some(gfx_hal::pso::DescriptorRangeDesc {
                ty: gfx_hal::pso::DescriptorType::UniformBuffer,
                count: 5,
            })
        )}.unwrap();

        let buffer = factory.create_buffer(
            UBERALIGN,
            BUFFER_FRAME_SIZE * MAX_FRAMES,
            (gfx_hal::buffer::Usage::UNIFORM|gfx_hal::buffer::Usage::INDIRECT|gfx_hal::buffer::Usage::VERTEX, MemoryUsageValue::Dynamic),
        ).unwrap();

        Ok(MeshRenderPipeline {
            descriptor_pool,
            buffer,
            sets: vec![None, None, None, None, None],
        })
    }

    fn prepare(
        &mut self,
        factory: &mut Factory<B>,
        set_layouts: &[B::DescriptorSetLayout],
        index: usize,
        scene: &Scene<B>,
    ) -> PrepareResult {
        unsafe {
            factory.upload_visible_buffer(
                &mut self.buffer,
                uniform_offset(index),
                &[UniformArgs {
                    pad: [0, 0, 0],
                    proj: scene.camera.proj.to_homogeneous(),
                    view: scene.camera.view.inverse().to_homogeneous(),
                    lights_count: scene.lights.len() as i32,
                    lights: {
                        let mut array = [Light { pad: 0.0, pos: nalgebra::Vector3::new(0.0, 0.0, 0.0), intencity: 0.0 }; MAX_LIGHTS];
                        let count = min(scene.lights.len(), 32);
                        array[..count].copy_from_slice(&scene.lights[..count]);
                        array
                    },
                }],
            ).unwrap()
        };

        unsafe {
            factory.upload_visible_buffer(
                &mut self.buffer,
                indirect_offset(index),
                &[DrawIndexedCommand {
                    index_count: scene.object_mesh.as_ref().unwrap().len(),
                    instance_count: scene.objects.len() as u32,
                    first_index: 0,
                    vertex_offset: 0,
                    first_instance: 0,
                }],
            ).unwrap()
        };

        unsafe {
            factory.upload_visible_buffer(
                &mut self.buffer,
                transforms_offset(index),
                &scene.objects[..],
            ).unwrap()
        };

        if self.sets[index].is_none() {
            unsafe { 
                let set = self.descriptor_pool.allocate_set(&set_layouts[0]).unwrap();
                factory.write_descriptor_sets(
                    Some(gfx_hal::pso::DescriptorSetWrite {
                        set: &set,
                        binding: 0,
                        array_offset: 0,
                        descriptors: Some(gfx_hal::pso::Descriptor::Buffer(
                            self.buffer.raw(),
                            Some(uniform_offset(index)) .. Some(uniform_offset(index) + UNIFORM_SIZE),
                        )),
                    }),
                );
                self.sets[index] = Some(set);
            }
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
        encoder.bind_graphics_descriptor_sets(
            layout,
            0,
            Some(self.sets[index].as_ref().unwrap()),
            std::iter::empty(),
        );
        assert!(scene.object_mesh.as_ref().unwrap().bind(&[PosColorNorm::VERTEX], &mut encoder).is_ok());
        encoder.bind_vertex_buffers(
            1,
            std::iter::once((self.buffer.raw(), transforms_offset(index))),
        );
        encoder.draw_indexed_indirect(
            self.buffer.raw(),
            indirect_offset(index),
            1,
            INDIRECT_SIZE as u32,
        );
    }

    fn dispose(mut self, factory: &mut Factory<B>, _aux: &mut Scene<B>) {
        unsafe {
            self.descriptor_pool.free_sets(self.sets.into_iter().filter_map(|s|s));
            factory.destroy_descriptor_pool(self.descriptor_pool);
        }
    }
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("triangle", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let mut factory: Factory<Backend> = Factory::new(config).unwrap();

    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop).unwrap();

    event_loop.poll_events(|_| ());

    let surface = factory.create_surface(window.into());

    let mut scene = Scene {
        camera: Camera {
            proj: nalgebra::Perspective3::new(surface.aspect(), 3.1415 / 4.0, 1.0, 200.0),
            view: nalgebra::Projective3::identity() * nalgebra::Translation3::new(0.0, 0.0, 10.0),
        },
        object_mesh: None,
        objects: vec![],
        lights: vec![
            Light {
                pad: 0.0,
                pos: nalgebra::Vector3::new(0.0, 0.0, 0.0),
                intencity: 10.0
            },
            Light {
                pad: 0.0,
                pos: nalgebra::Vector3::new(0.0, 20.0, -20.0),
                intencity: 140.0
            },
            Light {
                pad: 0.0,
                pos: nalgebra::Vector3::new(-20.0, 0.0, -60.0),
                intencity: 100.0
            },
            Light {
                pad: 0.0,
                pos: nalgebra::Vector3::new(20.0, -30.0, -100.0),
                intencity: 160.0
            },
        ],
    };

    let mut graph_builder = GraphBuilder::<Backend, Scene<Backend>>::new();

    let color = graph_builder.create_image(
        surface.kind(),
        1,
        factory.get_surface_format(&surface),
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

    let pass = graph_builder.add_node(
        MeshRenderPipeline::builder()
            .into_subpass()
            .with_color(color)
            .with_depth_stencil(depth)
            .into_pass()
    );

    graph_builder.add_node(
        PresentNode::builder(surface, color)
            .with_dependency(pass)
    );

    log::info!("{:#?}", scene);

    let mut graph = graph_builder.build(&mut factory, &mut scene).unwrap();

    let icosphere = genmesh::generators::IcoSphere::subdivide(4);
    let indices: Vec<_> = genmesh::Vertices::vertices(icosphere.indexed_polygon_iter()).map(|i| i as u32).collect();
    let vertices: Vec<_> = icosphere.shared_vertex_iter().map(|v| PosColorNorm {
        position: v.pos.into(),
        color: [
            (v.pos.x + 1.0) / 2.0,
            (v.pos.y + 1.0) / 2.0,
            (v.pos.z + 1.0) / 2.0,
            1.0,
        ].into(),
        normal: v.normal.into(),
    }).collect();

    scene.object_mesh = Some(Mesh::<Backend>::builder()
        .with_indices(&indices[..])
        .with_vertices(&vertices[..])
        .build(graph.node_queue(pass), &mut factory)
        .unwrap());

    let started = time::Instant::now();

    let mut frames = 0u64 ..;
    let mut rng = rand::thread_rng();
    let rxy = Uniform::new(-1.0, 1.0);
    let rz = Uniform::new(0.0, 185.0);

    let mut fpss = Vec::new();
    let mut checkpoint = started;

    while scene.objects.len() < MAX_OBJECTS {
        let start = frames.start;
        let from = scene.objects.len();
        for _ in &mut frames {
            event_loop.poll_events(|_| ());
            graph.run(&mut factory, &mut scene);

            let elapsed = checkpoint.elapsed();

            if scene.objects.len() < MAX_OBJECTS {
                scene.objects.push(
                    {
                        let z = rz.sample(&mut rng);
                        nalgebra::Transform3::identity() * nalgebra::Translation3::new(
                            rxy.sample(&mut rng) * (z + 10.0),
                            rxy.sample(&mut rng) * (z + 10.0),
                            -z,
                        )
                    }
                )
            }

            if elapsed > std::time::Duration::new(5, 0) || scene.objects.len() == MAX_OBJECTS {
                let frames = frames.start - start;
                let nanos = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;
                fpss.push((frames * 1_000_000_000 / nanos, from .. scene.objects.len()));
                checkpoint += elapsed;
                break;
            }
        }
    }

    log::info!("FPS: {:#?}", fpss);

    graph.dispose(&mut factory, &mut scene);
}

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
fn main() {
    panic!("Specify feature: { dx12, metal, vulkan }");
}
