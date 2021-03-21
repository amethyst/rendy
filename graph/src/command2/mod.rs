use std::sync::Arc;
use std::mem::MaybeUninit;
use std::hash::{Hash, Hasher};
use std::ops::{Index, Deref};

use dashmap::{DashMap, mapref::one::Ref};

use rendy_core::hal;
use hal::device::Device as _;

mod graphics_pipeline;
pub use graphics_pipeline::{HashableGraphicsPipelineDescr, HashableGraphicsPipelineTypes, HashablePrimitiveAssemblerDescr};

use crate::factory::Factory;
use crate::shader::{ShaderSourceSet, SpecConstantSet, ShaderId, ShaderSet, PipelineLayoutDescr, SpirvReflection};

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
    colors: Vec<hal::pass::AttachmentRef>,
    depth_stencil: Option<hal::pass::AttachmentRef>,
    inputs: Vec<hal::pass::AttachmentRef>,
    resolves: Vec<hal::pass::AttachmentRef>,
    preserves: Vec<hal::pass::AttachmentRef>,
}

#[derive(Debug, Clone)]
pub struct RenderPassKey {
    attachments: Vec<hal::pass::Attachment>,
    subpasses: Vec<SubpassDescKey>,
    dependencies: Vec<hal::pass::SubpassDependency>,
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
            dashmap::mapref::entry::Entry::Occupied(occupied) => *occupied.get(),
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                let built = key.source.build(factory, pipeline_layout, key.spec_constants.clone()).unwrap();
                let shader_id = built.shader_id();

                self.shader_sets.insert(shader_id, (built, reflection));
                vacant.insert(shader_id);

                shader_id
            },
        }
    }

    pub fn make_graphics_pipeline(
        &self,
        factory: &Factory<B>,
        key: HashableGraphicsPipelineDescr<CacheGraphicsPipelineTypes>
    ) -> GraphicsPipelineId {
        todo!()
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
