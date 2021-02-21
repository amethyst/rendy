use {
    std::marker::PhantomData,
    crate::{
        handle::{Handle, HasKey, HasValue},
        resource::Managed,
    },
    rendy_core::{
        Device,
        hal::{
            self,
            device::AllocationError,
            device::Device as DeviceTrait,
        },
    },
    rendy_resource::SamplerDesc,
};

pub type ManagedSampler<B> = Managed<SamplerMarker<B>>;
pub struct SamplerMarker<B>(PhantomData<B>) where B: hal::Backend;
pub type SamplerHandle<B> = Handle<SamplerMarker<B>>;

impl<B> HasKey for SamplerMarker<B> where B: hal::Backend {
    type Key = SamplerDesc;
}
impl<B> HasValue for SamplerMarker<B> where B: hal::Backend {
    type Value = ManagedSamplerData<B>;
}

pub struct ManagedSamplerData<B> where B: hal::Backend {
    key: SamplerDesc,
    raw: B::Sampler,
    _phantom: PhantomData<B>,
}

impl<B> ManagedSamplerData<B> where B: hal::Backend {

    pub fn create(device: &Device<B>, key: SamplerDesc) -> Result<Self, AllocationError> {
        let raw = unsafe {
            device.create_sampler(&key)?
        };
        let data = ManagedSamplerData {
            key,
            raw,
            _phantom: PhantomData,
        };
        Ok(data)
    }

}

impl<B> ManagedSampler<B> where B: hal::Backend {

    pub fn raw(&self) -> &B::Sampler {
        &self.inner.value.raw
    }

}
