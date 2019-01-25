//!
//! The mighty triangle example.
//! This examples shows colord triangle on white background.
//! Nothing fancy. Just prove that `rendy` works.
//! 

#![forbid(overflowing_literals)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(intra_doc_link_resolution_failure)]
#![warn(path_statements)]
#![warn(trivial_bounds)]
#![warn(type_alias_bounds)]
#![warn(unconditional_recursion)]
#![warn(unions_with_drop_fields)]
#![warn(while_true)]
#![warn(unused)]
#![warn(bad_style)]
#![warn(future_incompatible)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]

#![cfg_attr(not(any(feature = "dx12", feature = "metal", feature = "vulkan")), allow(unused))]

use rendy::{
    command::{RenderPassInlineEncoder, DrawIndexedCommand, QueueId, FamilyId},
    factory::{Config, Factory},
    frame::cirque::Cirque,
    graph::{Graph, GraphBuilder, render::*, present::PresentNode, NodeBuffer, NodeImage},
    memory::MemoryUsageValue,
    mesh::{AsVertex, PosColorNorm, Mesh, Transform, Position},
    shader::{Shader, StaticShaderInfo, ShaderKind, SourceLanguage},
    resource::buffer::Buffer,
    hal::{Device, pso::DescriptorPool},
};

use std::{ops::Range, time, mem::size_of};

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
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/mesh/shader.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );

    static ref FRAGMENT: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/mesh/shader.frag"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    );
}

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
struct Light {
    pos: nalgebra::Vector3<f32>,
    intencity: f32,
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
struct UniformArgs {
    proj: nalgebra::Matrix4<f32>,
    view: nalgebra::Matrix4<f32>,
    lights: [Light; MAX_LIGHTS],
    lights_count: i32,
}

#[derive(Debug)]
struct Camera {
    view: nalgebra::Projective3<f32>,
    proj: nalgebra::Perspective3<f32>,
}

#[derive(Debug)]
struct Scene<B: gfx_hal::Backend> {
    camera: Camera,
    object_mesh: Mesh<B>,
    objects: Vec<nalgebra::Transform3<f32>>,
    lights: Vec<Light>,
}

const MAX_LIGHTS: usize = 32;
const MAX_OBJECTS: usize = 1024;

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
struct MeshRenderPass<B: gfx_hal::Backend> {
    descriptor_pool: B::DescriptorPool,
    buffer: Buffer<B>,
    sets: Vec<Option<B::DescriptorSet>>,
}

impl<B> RenderPass<B, Scene<B>> for MeshRenderPass<B>
where
    B: gfx_hal::Backend,
{
    fn name() -> &'static str {
        "Mesh"
    }

    fn depth() -> bool { true }

    fn layouts() -> Vec<Layout> {
        vec![Layout {
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
        }]
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

    fn load_shader_sets<'a>(
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        _aux: &mut Scene<B>,
    ) -> Vec<gfx_hal::pso::GraphicsShaderSet<'a, B>> {
        storage.clear();

        log::trace!("Load shader module '{:#?}'", *VERTEX);
        storage.push(VERTEX.module(factory).unwrap());

        log::trace!("Load shader module '{:#?}'", *FRAGMENT);
        storage.push(FRAGMENT.module(factory).unwrap());

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

    fn build<'a>(
        factory: &mut Factory<B>,
        _aux: &mut Scene<B>,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
        sets: &[impl AsRef<[B::DescriptorSetLayout]>],
    ) -> Self {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].as_ref().len(), 1);
        
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

        MeshRenderPass {
            descriptor_pool,
            buffer,
            sets: vec![None, None, None, None, None],
        }
    }

    fn prepare(
        &mut self,
        factory: &mut Factory<B>,
        sets: &[impl AsRef<[B::DescriptorSetLayout]>],
        index: usize,
        scene: &Scene<B>,
    ) -> PrepareResult {
        unsafe {
            factory.upload_visible_buffer(
                &mut self.buffer,
                uniform_offset(index),
                &[UniformArgs {
                    proj: scene.camera.proj.to_homogeneous(),
                    view: scene.camera.view.inverse().to_homogeneous(),
                    lights_count: scene.lights.len() as i32,
                    lights: [Light { pos: nalgebra::Vector3::new(0.0, 0.0, 0.0), intencity: 0.0 }; MAX_LIGHTS],
                }],
            ).unwrap()
        };

        unsafe {
            factory.upload_visible_buffer(
                &mut self.buffer,
                indirect_offset(index),
                &[DrawIndexedCommand {
                    index_count: scene.object_mesh.len(),
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
                let set = self.descriptor_pool.allocate_set(&sets[0].as_ref()[0]).unwrap();
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
        layouts: &[B::PipelineLayout],
        pipelines: &[B::GraphicsPipeline],
        mut encoder: RenderPassInlineEncoder<'_, B>,
        index: usize,
        scene: &Scene<B>,
    ) {
        encoder.bind_graphics_pipeline(&pipelines[0]);
        encoder.bind_graphics_descriptor_sets(
            &layouts[0],
            0,
            Some(self.sets[index].as_ref().unwrap()),
            std::iter::empty(),
        );
        let vertices = scene.object_mesh.bind(&[PosColorNorm::VERTEX], &mut encoder);
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
fn run(event_loop: &mut EventsLoop, factory: &mut Factory<Backend>, mut graph: Graph<Backend, Scene<Backend>>, scene: &mut Scene<Backend>) -> Result<(), failure::Error> {

    let started = time::Instant::now();

    std::thread::spawn(move || {
        while started.elapsed() < time::Duration::new(30, 0) {
            std::thread::sleep(time::Duration::new(1, 0));
        }

        std::process::abort();
    });

    let mut frames = 0u64 ..;
    let mut elapsed = started.elapsed();

    for _ in &mut frames {
        event_loop.poll_events(|_| ());
        graph.run(factory, scene);

        elapsed = started.elapsed();
        if elapsed >= time::Duration::new(5, 0) {
            break;
        }
    }

    let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

    log::info!("Elapsed: {:?}. Frames: {}. FPS: {}", elapsed, frames.start, frames.start * 1_000_000_000 / elapsed_ns);

    graph.dispose(factory, scene);
    Ok(())
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

    let icosphere = genmesh::generators::IcoSphere::subdivide(5);
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

    let mesh = Mesh::<Backend>::builder()
        .with_indices(&indices[..])
        .with_vertices(&vertices[..])
        .build(QueueId(gfx_hal::queue::QueueFamilyId(0), 0), &mut factory)
        .unwrap();

    let surface = factory.create_surface(window.into());

    let mut scene = Scene {
        camera: Camera {
            proj: nalgebra::Perspective3::new(surface.aspect(), 3.1415 / 4.0, 1.0, 100.0),
            view: nalgebra::Projective3::identity() * nalgebra::Translation3::new(0.0, 0.0, 10.0),
        },
        object_mesh: mesh,
        objects: vec![
            nalgebra::Transform3::identity() * nalgebra::Translation3::new(0.0, 0.0, -3.0),
            nalgebra::Transform3::identity() * nalgebra::Translation3::new(-0.8, 0.0, -4.0),
        ],
        lights: Vec::new(),
    };

    log::info!("{:#?}", scene);

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
        MeshRenderPass::builder()
            .with_image(color)
            .with_image(depth)
    );

    graph_builder.add_node(
        PresentNode::builder(surface)
            .with_image(color)
            .with_dependency(pass)
    );

    let graph = graph_builder.build(&mut factory, &mut scene).unwrap();

    run(&mut event_loop, &mut factory, graph, &mut scene).unwrap();
}

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
fn main() {
    panic!("Specify feature: { dx12, metal, vulkan }");
}
