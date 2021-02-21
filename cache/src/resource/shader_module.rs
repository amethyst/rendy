use std::borrow::Cow;
use std::sync::Arc;
use std::marker::PhantomData;

use crate::{
    handle::{HasKey, HasValue},
    resource::{
        Managed,
    },
};

use {
    rendy_core::{
        Device,
        hal::{
            self,
            pso::Specialization,
            device::ShaderError,
            device::Device as DeviceTrait,
        },
    },
};

pub type ManagedShaderModule<B> = Managed<ShaderModuleMarker<B>>;
#[derive(Debug)]
pub struct ShaderModuleMarker<B>(PhantomData<B>) where B: hal::Backend;

impl<B> HasKey for ShaderModuleMarker<B> where B: hal::Backend {
    type Key = Arc<ShaderModuleKey>;
}
impl<B> HasValue for ShaderModuleMarker<B> where B: hal::Backend {
    type Value = ManagedShaderModuleData<B>;
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ShaderModuleKey {
    pub spirv: Spirv,
}

pub struct ManagedShaderModuleData<B>
where
    B: hal::Backend,
{
    raw: B::ShaderModule,
    key: Arc<ShaderModuleKey>,
    _phantom: PhantomData<B>,
}

impl<B> ManagedShaderModuleData<B>
where
    B: hal::Backend,
{

    pub fn create(device: &Device<B>, key: Arc<ShaderModuleKey>) -> Result<Self, ShaderError> {
        let raw = unsafe {
            device.create_shader_module(&key.spirv.0)?
        };
        let data = Self {
            raw,
            key,
            _phantom: PhantomData,
        };
        Ok(data)
    }

}

impl<B: hal::Backend> ManagedShaderModule<B> {

    pub fn raw(&self) -> &B::ShaderModule {
        &self.inner.value.raw
    }

}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spirv(Vec<u32>);
