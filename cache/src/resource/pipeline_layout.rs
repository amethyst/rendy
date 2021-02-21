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

use crate::{
    handle::{HasValue, HasKey},
    resource::{
        Managed,
        descriptor_set_layout::{ManagedDescriptorSetLayout, DescriptorSetLayoutHandle},
    },
};

pub type ManagedPipelineLayout<B> = Managed<PipelineLayoutMarker<B>>;
pub struct PipelineLayoutMarker<B>(PhantomData<B>) where B: hal::Backend;

impl<B> HasKey for PipelineLayoutMarker<B> where B: hal::Backend {
    type Key = Arc<PipelineLayoutKey<B>>;
}
impl<B> HasValue for PipelineLayoutMarker<B> where B: hal::Backend {
    type Value = ManagedPipelineLayoutData<B>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineLayoutKey<B> where B: hal::Backend {
    pub set_layouts: Vec<DescriptorSetLayoutHandle<B>>,
    pub push_constants: Vec<(ShaderStageFlags, Range<u32>)>,
}

pub struct ManagedPipelineLayoutData<B> where B: hal::Backend {
    key: Arc<PipelineLayoutKey<B>>,
    raw: B::PipelineLayout,
    set_layouts: Vec<ManagedDescriptorSetLayout<B>>,
}

impl<B> ManagedPipelineLayoutData<B> where B: hal::Backend {

    pub fn create(
        device: &Device<B>,
        key: Arc<PipelineLayoutKey<B>>,
        set_layouts: Vec<ManagedDescriptorSetLayout<B>>,
    ) -> Result<Self, OutOfMemory>
    {
        assert!(key.set_layouts.len() == set_layouts.len());
        for (slayout, key_slayout) in set_layouts
            .iter()
            .zip(key.set_layouts.iter())
        {
            assert!(slayout.handle() == *key_slayout);
            assert!(slayout.handle().device() == device.id());
        }

        let raw = unsafe {
            device.create_pipeline_layout(
                set_layouts.iter().map(|v| v.raw()),
                &key.push_constants,
            )?
        };

        let data = Self {
            raw,
            key,
            set_layouts,
        };

        Ok(data)
    }

}

impl<B: hal::Backend> ManagedPipelineLayout<B> {

    pub fn raw(&self) -> &B::PipelineLayout {
        &self.inner.value.raw
    }

}
