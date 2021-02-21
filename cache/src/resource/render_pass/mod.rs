use std::marker::PhantomData;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::ops::Range;

use rendy_core::{
    hal::{
        self,
        device::OutOfMemory,
        pso::ShaderStageFlags,
    },
    Device,
    hal::device::Device as DeviceTrait,
};
use rendy_resource::Layout;

use crate::{
    handle::{HasValue, HasKey},
    resource::{
        Managed,
        descriptor_set_layout::{ManagedDescriptorSetLayout, DescriptorSetLayoutHandle},
    },
};

mod render_pass_compatibility;
pub use render_pass_compatibility::RenderPassCompatibilityData;

pub type ManagedRenderPass<B> = Managed<RenderPassMarker<B>>;
pub struct RenderPassMarker<B>(PhantomData<B>) where B: hal::Backend;

impl<B> HasKey for RenderPassMarker<B> where B: hal::Backend {
    type Key = Arc<RenderPassKey>;
}
impl<B> HasValue for RenderPassMarker<B> where B: hal::Backend {
    type Value = ManagedRenderPassData<B>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPassKey {
    pub attachments: Vec<hal::pass::Attachment>,
    pub subpasses: Vec<SubpassDesc>,
    pub dependencies: Vec<hal::pass::SubpassDependency>,
}

pub struct ManagedRenderPassData<B> where B: hal::Backend {
    pub key: Arc<RenderPassKey>,
    pub compat: Arc<RenderPassCompatibilityData>,
    pub raw: B::RenderPass,
}

impl<B> ManagedRenderPassData<B> where B: hal::Backend {

    pub fn create(
        device: &Device<B>,
        key: Arc<RenderPassKey>,
    ) -> Result<Self, OutOfMemory>
    {
        let compat = RenderPassCompatibilityData::new(
            &key.attachments,
            &key.subpasses,
            &key.dependencies,
        );

        let raw = unsafe {
            device.create_render_pass(
                &key.attachments,
                key.subpasses
                   .iter()
                   .map(|v| {
                       hal::pass::SubpassDesc {
                           colors: &v.colors,
                           depth_stencil: v.depth_stencil.as_ref(),
                           inputs: &v.inputs,
                           resolves: &v.resolves,
                           preserves: &v.preserves,
                       }
                   }),
                &key.dependencies,
            )?
        };

        let data = Self {
            key,
            raw,
            compat: Arc::new(compat),
        };

        Ok(data)
    }

}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubpassDesc {
    pub depth_stencil: Option<(usize, Layout)>,
    pub colors: Vec<(usize, Layout)>,
    pub inputs: Vec<(usize, Layout)>,
    pub resolves: Vec<(usize, Layout)>,
    pub preserves: Vec<usize>,
}

impl<B: hal::Backend> ManagedRenderPass<B> {

    pub fn raw(&self) -> &B::RenderPass {
        &self.inner.value.raw
    }

    pub fn compat(&self) -> &Arc<RenderPassCompatibilityData> {
        &self.inner.value.compat
    }

}
