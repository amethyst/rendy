use std::marker::PhantomData;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::ops::Range;
use std::borrow::Cow;

use rendy_core::{
    hal::{
        self,
        pso::{ShaderStageFlags, CreationError},
    },
    Device,
    hal::device::Device as DeviceTrait,
};

use crate::{
    handle::{HasValue, HasKey},
    resource::{
        Managed,
        descriptor_set_layout::{ManagedDescriptorSetLayout, DescriptorSetLayoutHandle},
        render_pass::{ManagedRenderPass, RenderPassCompatibilityData},
        pipeline_layout::ManagedPipelineLayout,
        shader_module::ManagedShaderModule,
    },
};

pub type ManagedGraphicsPipeline<B> = Managed<GraphicsPipelineMarker<B>>;
pub struct GraphicsPipelineMarker<B>(PhantomData<B>) where B: hal::Backend;

impl<B> HasKey for GraphicsPipelineMarker<B> where B: hal::Backend {
    type Key = Arc<GraphicsPipelineKey<B>>;
}
impl<B> HasValue for GraphicsPipelineMarker<B> where B: hal::Backend {
    type Value = ManagedGraphicsPipelineData<B>;
}

#[derive(PartialEq, Eq, Hash)]
pub struct GraphicsPipelineKey<B> where B: hal::Backend {
    pub shaders: GraphicsShaderSet<B>,
    pub layout: ManagedPipelineLayout<B>,
    pub pass: Arc<RenderPassCompatibilityData>,
    pub subpass: usize,
    pub desc: GraphicsPipelineDesc,
}

pub struct ManagedGraphicsPipelineData<B> where B: hal::Backend {
    key: Arc<GraphicsPipelineKey<B>>,
    raw: B::GraphicsPipeline,
}

impl<B> ManagedGraphicsPipelineData<B> where B: hal::Backend {

    pub fn create(
        device: &Device<B>,
        key: Arc<GraphicsPipelineKey<B>>,
        pass: &ManagedRenderPass<B>,
        cache: Option<&B::PipelineCache>,
    ) -> Result<Self, CreationError>
    {
        assert!(&key.pass == pass.compat());
        let desc = hal::pso::GraphicsPipelineDesc {
            shaders: key.shaders.get_raw(),
            rasterizer: key.desc.rasterizer,
            vertex_buffers: key.desc.vertex_buffers.clone(),
            attributes: key.desc.attributes.clone(),
            input_assembler: key.desc.input_assembler.clone(),
            blender: key.desc.blender.clone(),
            depth_stencil: key.desc.depth_stencil,
            multisampling: key.desc.multisampling.clone(),
            baked_states: key.desc.baked_states.clone(),
            layout: key.layout.raw(),
            subpass: hal::pass::Subpass {
                main_pass: pass.raw(),
                index: key.subpass,
            },
            flags: key.desc.flags,
            parent: hal::pso::BasePipeline::None,
        };
        let raw = unsafe {
            device.create_graphics_pipeline(
                &desc,
                cache,
            )?
        };
        let data = ManagedGraphicsPipelineData {
            key,
            raw,
        };
        Ok(data)
    }

}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct GraphicsPipelineDesc {
    pub rasterizer: hal::pso::Rasterizer,
    pub vertex_buffers: Vec<hal::pso::VertexBufferDesc>,
    pub attributes: Vec<hal::pso::AttributeDesc>,
    pub input_assembler: hal::pso::InputAssemblerDesc,
    pub blender: hal::pso::BlendDesc,
    pub depth_stencil: hal::pso::DepthStencilDesc,
    pub multisampling: Option<hal::pso::Multisampling>,
    pub baked_states: hal::pso::BakedStates,
    pub flags: hal::pso::PipelineCreationFlags,
}

#[derive(Hash, PartialEq, Eq)]
pub struct EntryPoint<B>
where
    B: hal::Backend,
{
    module: ManagedShaderModule<B>,
    entry: String,
    specialization: hal::pso::Specialization<'static>,
    _phantom: PhantomData<B>,
}
impl<B> EntryPoint<B>
where
    B: hal::Backend,
{

    pub fn get_raw<'a>(&'a self) -> hal::pso::EntryPoint<'a, B> {
        hal::pso::EntryPoint {
            entry: &self.entry,
            module: self.module.raw(),
            specialization: hal::pso::Specialization {
                constants: match &self.specialization.constants {
                    Cow::Borrowed(borrow) => Cow::Borrowed(borrow),
                    Cow::Owned(own) => Cow::Borrowed(own),
                },
                data: match &self.specialization.data {
                    Cow::Borrowed(borrow) => Cow::Borrowed(borrow),
                    Cow::Owned(own) => Cow::Borrowed(own),
                },
            },
        }
    }

}

#[derive(Hash, PartialEq, Eq)]
pub struct GraphicsShaderSet<B>
where
    B: hal::Backend,
{
    vertex: EntryPoint<B>,
    hull: Option<EntryPoint<B>>,
    domain: Option<EntryPoint<B>>,
    geometry: Option<EntryPoint<B>>,
    fragment: Option<EntryPoint<B>>,
}

impl<B> GraphicsShaderSet<B>
where
    B: hal::Backend,
{

    pub fn get_raw<'a>(&'a self) -> hal::pso::GraphicsShaderSet<'a, B> {
        hal::pso::GraphicsShaderSet {
            vertex: self.vertex.get_raw(),
            hull: self.hull.as_ref().map(|v| v.get_raw()),
            domain: self.domain.as_ref().map(|v| v.get_raw()),
            geometry: self.geometry.as_ref().map(|v| v.get_raw()),
            fragment: self.fragment.as_ref().map(|v| v.get_raw()),
        }
    }

}
