use std::{borrow::Borrow, ops::DerefMut};

use descriptor::*;
use device::Device;
use shaders::ShaderStageFlags;

bitflags! {
    /// Flags to control descriptor set layout creatiion.
    #[repr(transparent)]
    pub struct DescriptorSetLayoutCreateFlags: u32 {
        /// Sets with this layout must not be allocated from pool.
        /// Descriptors are instead pushed by `command::Buffer::push_descriptor_set`.
        const PUSH_DESCRIPTOR = 0x00000001;

        /// Descriptor sets must be allocated from pool
        /// with `UPDATE_AFTER_BIND_BIT` bit set.
        const UPDATE_AFTER_BIND_POOL = 0x00000002;
    }
}

/// Single binding of the set layout.
#[derive(Clone, Copy, Debug)]
pub struct DescriptorSetLayoutBinding {
    binding: u32,
    descriptor_type: DescriptorType,
    descriptor_count: u32,
    stages: ShaderStageFlags,
}

/// Creation info for set layout.
#[derive(Clone, Debug)]
pub struct DescriptorSetLayoutCreateInfo<B> {
    flags: DescriptorSetLayoutCreateFlags,
    bindings: B,
}

/// Untyped set write.
#[allow(missing_debug_implementations)]
pub struct RawDescriptorSetWrite<'a, D: Device, W> {
    /// Set to write.
    pub set: &'a D::DescriptorSet,
    /// Binding index.
    pub binding: u32,

    /// Array element of the descriptor binding.
    pub element: u32,

    /// Iterator over `RawDescriptorWrite` values.
    pub writes: W,
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
