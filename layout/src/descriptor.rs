//!
//! A descriptor is an opaque data structure representing a shader resource.
//! For more info see Vulkan docs: (https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#descriptorsets)
//!
//! With `layout` user can define descriptors as fields for structures that represent descriptor sets.
//!

use device::Device;
use resource::image;
use std::ops::Range;

/// Type of the descriptor.
/// Every descriptor has a type.
/// Type must be specified during both layout creation and descriptor writing.
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum DescriptorType {
    /// Sampler descriptor.
    Sampler = 0,

    /// Image view combined with sampler.
    CombinedImageSampler = 1,

    /// Image view to use with sampler.
    SampledImage = 2,

    /// Image view for per-pixel access.
    StorageImage = 3,

    /// Buffer range with dynamic array of read-only texels.
    UniformTexelBuffer = 4,

    /// Buffer range with dynamic array of read-write texels.
    StorageTexelBuffer = 5,

    /// Buffer range with read-only uniform structure.
    UniformBuffer = 6,

    /// Buffer range with read-write uniform structure.
    StorageBuffer = 7,

    #[doc(hidden)]
    UniformBufferDynamic = 8,

    #[doc(hidden)]
    StorageBufferDynamic = 9,

    #[doc(hidden)]
    InputAttachment = 10,
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub enum RawDescriptorWrite<'a, D: Device> {
    Sampler(&'a D::Sampler),
    CombinedImageSampler(&'a D::Sampler, &'a D::ImageView, image::Layout),
    SampledImage(&'a D::ImageView, image::Layout),
    StorageImage(&'a D::ImageView, image::Layout),
    InputAttachment(&'a D::ImageView, image::Layout),
    UniformTexelBuffer(&'a D::BufferView),
    StorageTexelBuffer(&'a D::BufferView),
    UniformBuffer(&'a D::Buffer, Range<u64>),
    StorageBuffer(&'a D::Buffer, Range<u64>),
    UniformBufferDynamic(&'a D::Buffer, Range<u64>),
    StorageBufferDynamic(&'a D::Buffer, Range<u64>),
}

#[doc(hidden)]
pub trait Descriptor<D: Device, T> {
    fn descriptor_type() -> DescriptorType;
    fn write(&self) -> RawDescriptorWrite<'_, D>;
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct SamplerDescriptor;

impl<D> Descriptor<D, SamplerDescriptor> for D::Sampler
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::Sampler
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::Sampler(self)
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct CombinedImageSamplerDescriptor;

impl<D> Descriptor<D, CombinedImageSamplerDescriptor> for (D::Sampler, D::ImageView, image::Layout)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::CombinedImageSampler
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::CombinedImageSampler(&self.0, &self.1, self.2)
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct SampledImageDescriptor;

impl<D> Descriptor<D, SampledImageDescriptor> for (D::ImageView, image::Layout)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::SampledImage
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::SampledImage(&self.0, self.1)
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct StorageImageDescriptor;

impl<D> Descriptor<D, StorageImageDescriptor> for (D::ImageView, image::Layout)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::StorageImage
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::StorageImage(&self.0, self.1)
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct UniformTexelBufferDescriptor;

impl<D> Descriptor<D, UniformTexelBufferDescriptor> for D::BufferView
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::UniformTexelBuffer
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::UniformTexelBuffer(self)
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct StorageTexelBufferDescriptor;

impl<D> Descriptor<D, StorageTexelBufferDescriptor> for D::BufferView
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::StorageTexelBuffer
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::StorageTexelBuffer(self)
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct UniformBufferDescriptor;

impl<D> Descriptor<D, UniformBufferDescriptor> for (D::Buffer, Range<u64>)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::UniformBuffer
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::UniformBuffer(&self.0, self.1.clone())
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct StorageBufferDescriptor;

impl<D> Descriptor<D, StorageBufferDescriptor> for (D::Buffer, Range<u64>)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::StorageBuffer
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::StorageBuffer(&self.0, self.1.clone())
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct UniformBufferDynamicDescriptor;

impl<D> Descriptor<D, UniformBufferDynamicDescriptor> for (D::Buffer, Range<u64>)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::UniformBufferDynamic
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::UniformBufferDynamic(&self.0, self.1.clone())
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct StorageBufferDynamicDescriptor;

impl<D> Descriptor<D, StorageBufferDynamicDescriptor> for (D::Buffer, Range<u64>)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::StorageBufferDynamic
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::StorageBufferDynamic(&self.0, self.1.clone())
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct InputAttachmentDescriptor;

impl<D> Descriptor<D, InputAttachmentDescriptor> for (D::ImageView, image::Layout)
where
    D: Device,
{
    fn descriptor_type() -> DescriptorType {
        DescriptorType::InputAttachment
    }

    fn write(&self) -> RawDescriptorWrite<'_, D> {
        RawDescriptorWrite::InputAttachment(&self.0, self.1)
    }
}
