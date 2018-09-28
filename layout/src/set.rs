use std::{
    borrow::Borrow, marker::PhantomData, ops::{DerefMut, Range},
};

use resource::image;

use descriptor::*;
use device::Device;
use shaders::ShaderStageFlags;

bitflags! {
    /// Flags to control descriptor set layout creatiion.
    #[repr(transparent)]
    pub struct DescriptorSetLayoutCreateFlags: u32 {
        const PUSH_DESCRIPTOR = 0x00000001;
        const UPDATE_AFTER_BIND_POOL = 0x00000002;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DescriptorSetLayoutBinding {
    binding: u32,
    descriptor_type: DescriptorType,
    descriptor_count: u32,
    stages: ShaderStageFlags,
}

#[derive(Clone, Debug)]
pub struct DescriptorSetLayoutCreateInfo<B> {
    flags: DescriptorSetLayoutCreateFlags,
    bindings: B,
}

#[allow(missing_debug_implementations)]
pub struct RawDescriptorSetWrite<'a, D: Device> {
    set: &'a D::DescriptorSet,
    binding: u32,
    element: u32,
    writes: RawDescriptorWrite<'a, D>,
}

/// Abstract descriptor set descriptor.
pub trait DescriptorSet<D: Device> {
    /// Create info to create layout for the descriptor set descriptor.
    const LAYOUT_CREATE_INFO: DescriptorSetLayoutCreateInfo<&'static [DescriptorSetLayoutBinding]>;

    /// Cached descriptor set.
    type Cached: DerefMut<Target = Self> + Borrow<D::DescriptorSet>;

    /// Update descriptor set cache.
    fn update(cache: &Self::Cached, device: &D);
}
