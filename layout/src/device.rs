use memory::OutOfMemoryError;
use resource;
use std::{borrow::Borrow, fmt::Debug};

use descriptor::RawDescriptorWrite;
use pipeline::*;
use set::*;

/// Abstract device that can be used to create
/// * descriptor set layouts
/// * pipeline layouts
/// * pipelines
pub trait Device: resource::Device + Sized {
    /// Abstract descriptor set layout
    type DescriptorSetLayout: Debug + 'static;

    /// Abstract descriptor set.
    type DescriptorSet: Debug + 'static;

    /// Abstract pipeline layout
    type PipelineLayout: Debug + 'static;

    /// Abstract graphics pipeline
    type GraphicsPipeline: Debug + 'static;

    /// Create descriptor set layout.
    unsafe fn create_descriptor_set_layout<B>(
        &self,
        info: DescriptorSetLayoutCreateInfo<B>,
    ) -> Result<Self::DescriptorSetLayout, OutOfMemoryError>
    where
        B: IntoIterator,
        B::Item: Borrow<DescriptorSetLayoutBinding>;

    /// Create descriptor set layout.
    unsafe fn create_pipeline_layout<S, P>(
        &self,
        info: PipelineLayoutCreateInfo<S, P>,
    ) -> Result<Self::PipelineLayout, OutOfMemoryError>
    where
        S: IntoIterator,
        S::Item: Borrow<Self::DescriptorSetLayout>,
        P: IntoIterator,
        P::Item: Borrow<PushConstantRange>;

    /// Write descriptor set.
    unsafe fn write_descriptor_set<'a, W>(&self, write: RawDescriptorSetWrite<'_, Self, W>)
    where
        W: IntoIterator<Item = RawDescriptorWrite<'a, Self>>;
}
