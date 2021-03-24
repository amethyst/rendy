use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::mem::MaybeUninit;
use std::hash::{Hash, Hasher};
use std::ops::{Index, Deref};

use dashmap::{DashMap, mapref::{one::Ref, entry::Entry}};

use rendy_core::hal;
use hal::device::Device as _;

mod graphics_pipeline;
pub use graphics_pipeline::{HashableGraphicsPipelineDescr, HashableGraphicsPipelineTypes, HashablePrimitiveAssemblerDescr};

use crate::factory::Factory;
use crate::shader::{ShaderSourceSet, SpecConstantSet, ShaderId, ShaderSet, PipelineLayoutDescr, SpirvReflection};

const RENDER_PASS_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);
const GRAPHICS_PIPELINE_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

//#[derive(Debug)]
//pub struct PipelineCache<B: hal::Backend> {
//    pub device: Arc<B::Device>,
//    pub raw: MaybeUninit<B::PipelineCache>,
//}
//impl<B: hal::Backend> PipelineCache<B> {
//
//    pub fn new(device: Arc<B::Device>, data: Option<&[u8]>) -> Result<Self, hal::device::OutOfMemory> {
//        let raw = unsafe { device.create_pipeline_cache(data)? };
//        Ok(Self {
//            device,
//            raw: MaybeUninit::new(raw),
//        })
//    }
//
//}
//impl<B: hal::Backend> Drop for PipelineCache<B> {
//    fn drop(&mut self) {
//        unsafe { self.device.destroy_pipeline_cache(self.raw.assume_init_read()) }
//    }
//}

#[derive(Debug)]
pub struct CacheGraphicsPipelineTypes;
impl HashableGraphicsPipelineTypes for CacheGraphicsPipelineTypes {
    type Program = ShaderId;
    type Subpass = (RenderPassId, hal::pass::SubpassId);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShaderSetKey {
    pub source: ShaderSourceSet,
    pub spec_constants: SpecConstantSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubpassDescKey {
    pub colors: Vec<hal::pass::AttachmentRef>,
    pub depth_stencil: Option<hal::pass::AttachmentRef>,
    pub inputs: Vec<hal::pass::AttachmentRef>,
    pub resolves: Vec<hal::pass::AttachmentRef>,
    pub preserves: Vec<hal::pass::AttachmentId>,
}

#[derive(Debug, Clone)]
pub struct RenderPassKey {
    pub attachments: Vec<hal::pass::Attachment>,
    pub subpasses: Vec<SubpassDescKey>,
    pub dependencies: Vec<hal::pass::SubpassDependency>,
}
impl PartialEq for RenderPassKey {
    fn eq(&self, rhs: &Self) -> bool {
        self.attachments == rhs.attachments
            && self.subpasses == rhs.subpasses
            && self.dependencies.len() == rhs.dependencies.len()
            && self.dependencies.iter().zip(rhs.dependencies.iter()).all(|(l, r)| {
                l.passes == r.passes
                    && l.stages == r.stages
                    && l.accesses == r.accesses
                    && l.flags == r.flags
            })
    }
}
impl Eq for RenderPassKey {}
impl Hash for RenderPassKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.attachments.hash(state);
        self.subpasses.hash(state);

        state.write_usize(self.dependencies.len());
        for dep in self.dependencies.iter() {
            dep.passes.hash(state);
            dep.stages.hash(state);
            dep.accesses.hash(state);
            dep.flags.hash(state);
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineId(usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderPassId(usize);

pub struct Cache<B: hal::Backend> {
    pipeline_cache: B::PipelineCache,

    shader_set_keys: DashMap<Arc<ShaderSetKey>, ShaderId>,
    shader_sets: DashMap<ShaderId, (ShaderSet<B>, SpirvReflection)>,

    graphics_pipeline_keys: DashMap<Arc<graphics_pipeline::HashableGraphicsPipelineDescr<CacheGraphicsPipelineTypes>>, GraphicsPipelineId>,
    graphics_pipelines: DashMap<GraphicsPipelineId, B::GraphicsPipeline>,

    render_pass_keys: DashMap<Arc<RenderPassKey>, RenderPassId>,
    render_passes: DashMap<RenderPassId, B::RenderPass>,
}

impl<B: hal::Backend> Cache<B> {

    pub fn new(factory: &Factory<B>) -> Self {
        let pipeline_cache = unsafe {
            factory.create_pipeline_cache(None).unwrap()
        };
        Self {
            pipeline_cache,

            shader_set_keys: DashMap::new(),
            shader_sets: DashMap::new(),

            graphics_pipeline_keys: DashMap::new(),
            graphics_pipelines: DashMap::new(),

            render_pass_keys: DashMap::new(),
            render_passes: DashMap::new(),
        }
    }

    pub fn make_shader_set(
        &self,
        factory: &Factory<B>,
        key: Arc<ShaderSetKey>,
        reflection: SpirvReflection,
    ) -> ShaderId {
        let pipeline_layout = PipelineLayoutDescr::from_reflect(&reflection);

        let entity = self.shader_set_keys.entry(key.clone());

        match entity {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                let built = key.source.build(factory, pipeline_layout, key.spec_constants.clone()).unwrap();
                let shader_id = built.shader_id();

                self.shader_sets.insert(shader_id, (built, reflection));
                vacant.insert(shader_id);

                shader_id
            },
        }
    }

    pub fn make_render_pass(
        &self,
        factory: &Factory<B>,
        key: Arc<RenderPassKey>,
    ) -> RenderPassId {
        let entity = self.render_pass_keys.entry(key.clone());

        match entity {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                let render_pass = unsafe {
                    factory.device().create_render_pass(
                        key.attachments.iter().cloned(),
                        key.subpasses.iter().map(|subpass| {
                            hal::pass::SubpassDesc {
                                colors: &subpass.colors,
                                depth_stencil: subpass.depth_stencil.as_ref(),
                                inputs: &subpass.inputs,
                                resolves: &subpass.resolves,
                                preserves: &subpass.preserves,
                            }
                        }),
                        key.dependencies.iter().cloned(),
                    ).unwrap()
                };

                let render_pass_id = RenderPassId(RENDER_PASS_ID_COUNTER.fetch_add(1, Ordering::Relaxed));

                self.render_passes.insert(render_pass_id, render_pass);
                vacant.insert(render_pass_id);

                render_pass_id
            },
        }
    }

    pub fn make_graphics_pipeline(
        &self,
        factory: &Factory<B>,
        key: Arc<HashableGraphicsPipelineDescr<CacheGraphicsPipelineTypes>>
    ) -> GraphicsPipelineId {
        let entity = self.graphics_pipeline_keys.entry(key.clone());

        match entity {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                let shader_set_val = self.shader_sets.get(&key.program).unwrap();
                let shader_set = &shader_set_val.0;
                let reflection = &shader_set_val.1;

                let (render_pass_id, subpass_idx) = key.subpass;
                let render_pass = self.render_passes.get(&render_pass_id).unwrap();

                let (vert_elements, vert_stride, vert_rate) = reflection
                    .attributes_range(..)
                    .unwrap()
                    .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex);

                let buffers = [
                    hal::pso::VertexBufferDesc {
                        binding: 0,
                        stride: vert_stride,
                        rate: vert_rate,
                    }
                ];

                let attributes = vert_elements.iter().enumerate().map(|(idx, elem)| {
                    hal::pso::AttributeDesc {
                        location: idx as u32,
                        binding: 0,
                        element: *elem,
                    }
                }).collect::<Vec<_>>();

                let primitive_assembler = match &key.primitive_assembler {
                    HashablePrimitiveAssemblerDescr::Vertex { input_assembler, tessellation, geometry } => {
                        hal::pso::PrimitiveAssemblerDesc::Vertex {
                            // TODO multiple buffers
                            buffers: &buffers,
                            attributes: &attributes,
                            input_assembler: input_assembler.clone(),
                            vertex: shader_set.raw_vertex().unwrap().unwrap(),
                            tessellation: if *tessellation {
                                Some((
                                    shader_set.raw_hull().unwrap().unwrap(),
                                    shader_set.raw_domain().unwrap().unwrap(),
                                ))
                            } else {
                                None
                            },
                            geometry: if *geometry {
                                Some(shader_set.raw_geometry().unwrap().unwrap())
                            } else {
                                None
                            },
                        }
                    },
                    HashablePrimitiveAssemblerDescr::Mesh { .. } => todo!(),
                };

                let desc = hal::pso::GraphicsPipelineDesc {
                    label: key.label.as_ref().map(|v| &**v),
                    primitive_assembler,
                    rasterizer: key.rasterizer,
                    fragment: if key.fragment {
                        Some(shader_set.raw_fragment().unwrap().unwrap())
                    } else {
                        None
                    },
                    blender: key.blender.clone(),
                    depth_stencil: key.depth_stencil,
                    multisampling: key.multisampling.clone(),
                    baked_states: hal::pso::BakedStates::default(),
                    layout: shader_set.pipeline_layout().raw(),
                    subpass: hal::pass::Subpass {
                        index: subpass_idx,
                        main_pass: &*render_pass,
                    },
                    flags: hal::pso::PipelineCreationFlags::empty(),
                    parent: hal::pso::BasePipeline::None,
                };

                let graphics_pipeline = unsafe {
                    factory.create_graphics_pipeline(&desc, Some(&self.pipeline_cache)).unwrap()
                };

                let graphics_pipeline_id = GraphicsPipelineId(GRAPHICS_PIPELINE_ID_COUNTER.fetch_add(1, Ordering::Relaxed));

                self.graphics_pipelines.insert(graphics_pipeline_id, graphics_pipeline);
                vacant.insert(graphics_pipeline_id);

                graphics_pipeline_id
            }
        }
    }

    pub fn get_graphics_pipeline(&self, id: GraphicsPipelineId) -> GraphicsPipelineRef<B> {
        GraphicsPipelineRef {
            reference: self.graphics_pipelines.get(&id).unwrap(),
        }
    }

}

pub struct GraphicsPipelineRef<'a, B: hal::Backend> {
    reference: Ref<'a, GraphicsPipelineId, B::GraphicsPipeline>,
}
impl<'a, B: hal::Backend> Deref for GraphicsPipelineRef<'a, B> {
    type Target = B::GraphicsPipeline;
    fn deref(&self) -> &B::GraphicsPipeline {
        &*self.reference
    }
}
