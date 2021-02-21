use std::marker::PhantomData;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use rendy_core::{
    hal::{
        self,
        device::OutOfMemory,
    },
    Device,
    hal::device::Device as DeviceTrait,
};
use rendy_resource::{CreationError, SubresourceRange, ImageViewInfo};
use rendy_descriptor::DescriptorSetLayoutBinding;

use crate::{
    handle::{Handle, HasValue, HasKey},
    resource::{
        Managed,
        sampler::{ManagedSampler, SamplerHandle},
    },
};

pub type ManagedDescriptorSetLayout<B> = Managed<DescriptorSetLayoutMarker<B>>;
pub struct DescriptorSetLayoutMarker<B>(PhantomData<B>) where B: hal::Backend;
pub type DescriptorSetLayoutHandle<B> = Handle<DescriptorSetLayoutMarker<B>>;

impl<B> HasKey for DescriptorSetLayoutMarker<B> where B: hal::Backend {
    type Key = Arc<DescriptorSetLayoutKey<B>>;
}
impl<B> HasValue for DescriptorSetLayoutMarker<B> where B: hal::Backend {
    type Value = ManagedDescriptorSetLayoutData<B>;
}

pub struct DescriptorSetLayoutKey<B> where B: hal::Backend {
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    pub immutable_samplers: Vec<SamplerHandle<B>>,
}
impl<B: hal::Backend> PartialEq for DescriptorSetLayoutKey<B> {
    fn eq(&self, rhs: &Self) -> bool {
        self.bindings == rhs.bindings
            && self.immutable_samplers == rhs.immutable_samplers
    }
}
impl<B: hal::Backend> Eq for DescriptorSetLayoutKey<B> {}
impl<B: hal::Backend> Hash for DescriptorSetLayoutKey<B> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.bindings.hash(hasher);
        self.immutable_samplers.hash(hasher);
    }
}

pub struct ManagedDescriptorSetLayoutData<B> where B: hal::Backend {
    raw: B::DescriptorSetLayout,
    key: Arc<DescriptorSetLayoutKey<B>>,
    immutable_samplers: Vec<ManagedSampler<B>>,
}

impl<B> ManagedDescriptorSetLayoutData<B> where B: hal::Backend {

    pub fn create(
        device: &Device<B>,
        key: Arc<DescriptorSetLayoutKey<B>>,
        immutable_samplers: Vec<ManagedSampler<B>>,
    ) -> Result<Self, OutOfMemory>
    {
        assert!(key.immutable_samplers.len() == immutable_samplers.len());
        for (sampler, key_sampler) in immutable_samplers
            .iter()
            .zip(key.immutable_samplers.iter())
        {
            assert!(sampler.handle() == *key_sampler);
            assert!(sampler.handle().device() == device.id());
        }

        let raw = unsafe {
            device.create_descriptor_set_layout(
                &key.bindings,
                immutable_samplers.iter().map(|v| v.raw()),
            )?
        };

        let data = Self {
            raw,
            key,
            immutable_samplers,
        };

        Ok(data)
    }

}

impl<B> ManagedDescriptorSetLayout<B> where B: hal::Backend {

    pub fn raw(&self) -> &B::DescriptorSetLayout {
        &self.inner.value.raw
    }

}
