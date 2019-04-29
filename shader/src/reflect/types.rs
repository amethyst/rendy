//! Using spirv-reflect-rs for reflection.
//!

use gfx_hal::format::Format;
use spirv_reflect::types::*;
use std::collections::HashMap;

/// Workaround extension trait copy of std::convert::From, for simple conversion from spirv-reflect types to gfx_hal types
pub(crate) trait ReflectInto<T>: Sized {
    /// Attempts to perform a conversion from the provided type into this type
    fn reflect_into(&self) -> Result<T, failure::Error> {
        Err(failure::format_err!("Unsupported conversion type"))
    }
}

impl ReflectInto<Format> for ReflectFormat {
    fn reflect_into(&self) -> Result<Format, failure::Error> {
        match self {
            ReflectFormat::Undefined => Err(failure::format_err!("Undefined Format")),
            ReflectFormat::R32_UINT => Ok(Format::R32Uint),
            ReflectFormat::R32_SINT => Ok(Format::R32Int),
            ReflectFormat::R32_SFLOAT => Ok(Format::R32Float),
            ReflectFormat::R32G32_UINT => Ok(Format::Rg32Uint),
            ReflectFormat::R32G32_SINT => Ok(Format::Rg32Int),
            ReflectFormat::R32G32_SFLOAT => Ok(Format::Rg32Float),
            ReflectFormat::R32G32B32_UINT => Ok(Format::Rgb32Uint),
            ReflectFormat::R32G32B32_SINT => Ok(Format::Rgb32Int),
            ReflectFormat::R32G32B32_SFLOAT => Ok(Format::Rgb32Float),
            ReflectFormat::R32G32B32A32_UINT => Ok(Format::Rgb32Uint),
            ReflectFormat::R32G32B32A32_SINT => Ok(Format::Rgb32Int),
            ReflectFormat::R32G32B32A32_SFLOAT => Ok(Format::Rgb32Float),
        }
    }
}

pub(crate) fn type_element_format(
    flags: ReflectTypeFlags,
    traits: &ReflectTypeDescriptionTraits,
) -> Result<Format, failure::Error> {
    enum NumTy {
        SInt,
        UInt,
        Float,
    }

    let num_ty = if flags.contains(ReflectTypeFlags::INT) {
        match traits.numeric.scalar.signedness {
            0 => NumTy::UInt,
            1 => NumTy::SInt,
            _ => {
                failure::bail!(
                    "Unrecognized numeric signedness {:?}",
                    traits.numeric.scalar.signedness
                );
            }
        }
    } else if flags.contains(ReflectTypeFlags::FLOAT) {
        NumTy::Float
    } else {
        failure::bail!("Unrecognized numeric type with flags {:?}", flags);
    };

    let current_type = match (num_ty, traits.numeric.scalar.width) {
        (NumTy::SInt, 8) => Format::R8Int,
        (NumTy::SInt, 16) => Format::R16Int,
        (NumTy::SInt, 32) => Format::R32Int,
        (NumTy::SInt, 64) => Format::R64Int,
        (NumTy::UInt, 8) => Format::R8Uint,
        (NumTy::UInt, 16) => Format::R16Uint,
        (NumTy::UInt, 32) => Format::R32Uint,
        (NumTy::UInt, 64) => Format::R64Uint,
        (NumTy::Float, 32) => Format::R32Float,
        (NumTy::Float, 64) => Format::R64Float,
        _ => {
            failure::bail!(
                "Unrecognized numeric type with width {}",
                traits.numeric.scalar.width
            );
        }
    };

    if traits.numeric.vector.component_count > 1 {
        Ok(
            match (current_type, traits.numeric.vector.component_count) {
                (Format::R8Int, 2) => Format::Rg8Int,
                (Format::R8Int, 3) => Format::Rgb8Int,
                (Format::R8Int, 4) => Format::Rgba8Int,
                (Format::R16Int, 2) => Format::Rg16Int,
                (Format::R16Int, 3) => Format::Rgb16Int,
                (Format::R16Int, 4) => Format::Rgba16Int,
                (Format::R32Int, 2) => Format::Rg32Int,
                (Format::R32Int, 3) => Format::Rgb32Int,
                (Format::R32Int, 4) => Format::Rgba32Int,
                (Format::R64Int, 2) => Format::Rg64Int,
                (Format::R64Int, 3) => Format::Rgb64Int,
                (Format::R64Int, 4) => Format::Rgba64Int,
                (Format::R8Uint, 2) => Format::Rg8Uint,
                (Format::R8Uint, 3) => Format::Rgb8Uint,
                (Format::R8Uint, 4) => Format::Rgba8Uint,
                (Format::R16Uint, 2) => Format::Rg16Uint,
                (Format::R16Uint, 3) => Format::Rgb16Uint,
                (Format::R16Uint, 4) => Format::Rgba16Uint,
                (Format::R32Uint, 2) => Format::Rg32Uint,
                (Format::R32Uint, 3) => Format::Rgb32Uint,
                (Format::R32Uint, 4) => Format::Rgba32Uint,
                (Format::R64Uint, 2) => Format::Rg64Uint,
                (Format::R64Uint, 3) => Format::Rgb64Uint,
                (Format::R64Uint, 4) => Format::Rgba64Uint,
                (Format::R32Float, 2) => Format::Rg32Float,
                (Format::R32Float, 3) => Format::Rgb32Float,
                (Format::R32Float, 4) => Format::Rgba32Float,
                (Format::R64Float, 2) => Format::Rg64Float,
                (Format::R64Float, 3) => Format::Rgb64Float,
                (Format::R64Float, 4) => Format::Rgba64Float,
                _ => {
                    failure::bail!(
                        "Unrecognized numeric array with component count {}",
                        traits.numeric.vector.component_count
                    );
                }
            },
        )
    } else {
        Ok(current_type)
    }
}

impl ReflectInto<gfx_hal::pso::Element<Format>> for ReflectTypeDescription {
    fn reflect_into(&self) -> Result<gfx_hal::pso::Element<Format>, failure::Error> {
        let format = type_element_format(self.type_flags, &self.traits)?;
        Ok(gfx_hal::pso::Element {
            format: format,
            offset: 0,
        })
    }
}

impl ReflectInto<gfx_hal::pso::AttributeDesc> for ReflectInterfaceVariable {
    fn reflect_into(&self) -> Result<gfx_hal::pso::AttributeDesc, failure::Error> {
        // An attribute is not an image format
        Ok(gfx_hal::pso::AttributeDesc {
            location: self.location,
            binding: self.location,
            element: self
                .type_description
                .as_ref()
                .ok_or_else(|| failure::format_err!("Unable to reflect vertex element"))?
                .reflect_into()?,
        })
    }
}

// Descriptor Sets
//

impl ReflectInto<gfx_hal::pso::DescriptorType> for ReflectDescriptorType {
    fn reflect_into(&self) -> Result<gfx_hal::pso::DescriptorType, failure::Error> {
        use gfx_hal::pso::DescriptorType;
        use ReflectDescriptorType::*;

        match *self {
            Sampler => Ok(DescriptorType::Sampler),
            CombinedImageSampler => Ok(DescriptorType::CombinedImageSampler),
            SampledImage => Ok(DescriptorType::SampledImage),
            StorageImage => Ok(DescriptorType::StorageImage),
            UniformTexelBuffer => Ok(DescriptorType::UniformTexelBuffer),
            StorageTexelBuffer => Ok(DescriptorType::StorageTexelBuffer),
            UniformBuffer => Ok(DescriptorType::UniformBuffer),
            StorageBuffer => Ok(DescriptorType::StorageBuffer),
            UniformBufferDynamic => Ok(DescriptorType::UniformBufferDynamic),
            StorageBufferDynamic => Ok(DescriptorType::StorageBufferDynamic),
            InputAttachment => Ok(DescriptorType::InputAttachment),
            AccelerationStructureNV => Err(failure::format_err!(
                "We cant handle AccelerationStructureNV descriptor type"
            )),
            Undefined => Err(failure::format_err!(
                "We cant handle undefined descriptor types"
            )),
        }
    }
}

impl ReflectInto<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>> for ReflectDescriptorSet {
    fn reflect_into(
        &self,
    ) -> Result<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>, failure::Error> {
        self.bindings
            .iter()
            .map(|desc| desc.reflect_into())
            .collect::<Result<Vec<_>, _>>()
    }
}
impl ReflectInto<gfx_hal::pso::DescriptorSetLayoutBinding> for ReflectDescriptorBinding {
    fn reflect_into(&self) -> Result<gfx_hal::pso::DescriptorSetLayoutBinding, failure::Error> {
        Ok(gfx_hal::pso::DescriptorSetLayoutBinding {
            binding: self.binding,
            ty: self.descriptor_type.reflect_into()?,
            count: self.count as usize,
            stage_flags: gfx_hal::pso::ShaderStageFlags::VERTEX,
            immutable_samplers: false, // TODO: how to determine this?
        })
    }
}

pub(crate) fn convert_push_constant(
    stage: gfx_hal::pso::ShaderStageFlags,
    variable: &ReflectBlockVariable,
) -> Result<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>), failure::Error> {
    Ok((
        stage,
        variable.offset..variable.offset / 4 + variable.size / 4,
    ))
}

pub(crate) fn convert_stage(stage: ReflectShaderStageFlags) -> gfx_hal::pso::ShaderStageFlags {
    let mut bits = gfx_hal::pso::ShaderStageFlags::empty();

    if stage.contains(ReflectShaderStageFlags::VERTEX) {
        bits |= gfx_hal::pso::ShaderStageFlags::VERTEX;
    }
    if stage.contains(ReflectShaderStageFlags::FRAGMENT) {
        bits |= gfx_hal::pso::ShaderStageFlags::FRAGMENT;
    }
    if stage.contains(ReflectShaderStageFlags::GEOMETRY) {
        bits |= gfx_hal::pso::ShaderStageFlags::GEOMETRY;
    }
    if stage.contains(ReflectShaderStageFlags::COMPUTE) {
        bits |= gfx_hal::pso::ShaderStageFlags::COMPUTE;
    }
    if stage.contains(ReflectShaderStageFlags::TESSELLATION_CONTROL) {
        bits |= gfx_hal::pso::ShaderStageFlags::HULL;
    }
    if stage.contains(ReflectShaderStageFlags::TESSELLATION_EVALUATION) {
        bits |= gfx_hal::pso::ShaderStageFlags::DOMAIN;
    }

    bits
}

pub(crate) fn generate_attributes(
    attributes: Vec<ReflectInterfaceVariable>,
) -> Result<HashMap<(String, u8), gfx_hal::pso::AttributeDesc>, failure::Error> {
    let mut out_attributes = HashMap::new();

    for attribute in &attributes {
        let reflected: gfx_hal::pso::AttributeDesc = attribute.reflect_into()?;
        if attribute.array.dims.is_empty() {
            out_attributes.insert((attribute.name.clone(), 0), reflected);
        } else {
            for n in 0..attribute.array.dims[0] {
                let mut clone = reflected.clone();
                clone.location += n;
                out_attributes.insert((attribute.name.clone(), n as u8), clone);
            }
        }
    }

    Ok(out_attributes)
}
