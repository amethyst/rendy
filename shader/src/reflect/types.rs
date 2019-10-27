//! Using spirv-reflect-rs for reflection.
//!

use rendy_core::hal::format::Format;
use spirv_reflect::types::*;
use std::collections::HashMap;

/// A type reflection error.
#[derive(Copy, Clone, Debug)]
pub enum ReflectTypeError {
    /// Tried reflecting an undefined format.
    UndefinedFormat,
    /// The conversion isn't supported.
    UnsupportedConversion,
    /// An unrecognized numeric sign has been encountered.
    UnrecognizedNumericSignedness(u32),
    /// Unrecognized numeric flags have been encountered.
    UnrecognizedNumericTypeFlags(ReflectTypeFlags),
    /// An unrecognized numeric width has been encountered.
    UnrecognizedNumericTypeWidth(u32),
    /// An unrecognized array count for the format has been encountered.
    UnrecognizedNumericArrayCount(Format, u32),
    /// A vertex element could not be reflected.
    VertexElement,
    /// A `AccelerationStructureNV` descriptor type has been encountered which cannot be handled.
    UnhandledAccelerationStructureNV,
    /// An undefined descriptor type has been encountered which cannot be handled.
    UnhandledUndefined,
}

impl std::error::Error for ReflectTypeError {}
impl std::fmt::Display for ReflectTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ReflectTypeError::UndefinedFormat => write!(f, "undefined format"),
            ReflectTypeError::UnsupportedConversion => write!(f, "unsupported conversion type"),
            ReflectTypeError::UnrecognizedNumericSignedness(sign) => {
                write!(f, "unrecognized numeric signedness {}", sign)
            }
            ReflectTypeError::UnrecognizedNumericTypeFlags(flags) => {
                write!(f, "unrecognized numeric type with flags {:?}", flags)
            }
            ReflectTypeError::UnrecognizedNumericTypeWidth(width) => {
                write!(f, "unrecognized numeric type with width {}", width)
            }
            ReflectTypeError::UnrecognizedNumericArrayCount(format, count) => write!(
                f,
                "unrecognized numeric array with format {:?} and component count {}",
                format, count
            ),
            ReflectTypeError::VertexElement => write!(f, "Unable to reflect vertex element"),
            ReflectTypeError::UnhandledAccelerationStructureNV => {
                write!(f, "We cant handle AccelerationStructureNV descriptor type")
            }
            ReflectTypeError::UnhandledUndefined => {
                write!(f, "We cant handle undefined descriptor types")
            }
        }
    }
}

/// Workaround extension trait copy of std::convert::From, for simple conversion from spirv-reflect types to rendy_core::hal types
pub(crate) trait ReflectInto<T>: Sized {
    /// Attempts to perform a conversion from the provided type into this type
    fn reflect_into(&self) -> Result<T, ReflectTypeError> {
        Err(ReflectTypeError::UnsupportedConversion)
    }
}

impl ReflectInto<Format> for ReflectFormat {
    fn reflect_into(&self) -> Result<Format, ReflectTypeError> {
        match self {
            ReflectFormat::Undefined => Err(ReflectTypeError::UndefinedFormat),
            ReflectFormat::R32_UINT => Ok(Format::R32Uint),
            ReflectFormat::R32_SINT => Ok(Format::R32Sint),
            ReflectFormat::R32_SFLOAT => Ok(Format::R32Sfloat),
            ReflectFormat::R32G32_UINT => Ok(Format::Rg32Uint),
            ReflectFormat::R32G32_SINT => Ok(Format::Rg32Sint),
            ReflectFormat::R32G32_SFLOAT => Ok(Format::Rg32Sfloat),
            ReflectFormat::R32G32B32_UINT => Ok(Format::Rgb32Uint),
            ReflectFormat::R32G32B32_SINT => Ok(Format::Rgb32Sint),
            ReflectFormat::R32G32B32_SFLOAT => Ok(Format::Rgb32Sfloat),
            ReflectFormat::R32G32B32A32_UINT => Ok(Format::Rgb32Uint),
            ReflectFormat::R32G32B32A32_SINT => Ok(Format::Rgb32Sint),
            ReflectFormat::R32G32B32A32_SFLOAT => Ok(Format::Rgb32Sfloat),
        }
    }
}

pub(crate) fn type_element_format(
    flags: ReflectTypeFlags,
    traits: &ReflectTypeDescriptionTraits,
) -> Result<Format, ReflectTypeError> {
    enum NumTy {
        SInt,
        UInt,
        Float,
    }

    let num_ty = if flags.contains(ReflectTypeFlags::INT) {
        match traits.numeric.scalar.signedness {
            0 => NumTy::UInt,
            1 => NumTy::SInt,
            unk => return Err(ReflectTypeError::UnrecognizedNumericSignedness(unk)),
        }
    } else if flags.contains(ReflectTypeFlags::FLOAT) {
        NumTy::Float
    } else {
        return Err(ReflectTypeError::UnrecognizedNumericTypeFlags(flags));
    };

    let current_type = match (num_ty, traits.numeric.scalar.width) {
        (NumTy::SInt, 8) => Format::R8Sint,
        (NumTy::SInt, 16) => Format::R16Sint,
        (NumTy::SInt, 32) => Format::R32Sint,
        (NumTy::SInt, 64) => Format::R64Sint,
        (NumTy::UInt, 8) => Format::R8Uint,
        (NumTy::UInt, 16) => Format::R16Uint,
        (NumTy::UInt, 32) => Format::R32Uint,
        (NumTy::UInt, 64) => Format::R64Uint,
        (NumTy::Float, 32) => Format::R32Sfloat,
        (NumTy::Float, 64) => Format::R64Sfloat,
        (_, width) => return Err(ReflectTypeError::UnrecognizedNumericTypeWidth(width)),
    };

    if traits.numeric.vector.component_count > 1 {
        Ok(
            match (current_type, traits.numeric.vector.component_count) {
                (Format::R8Sint, 2) => Format::Rg8Sint,
                (Format::R8Sint, 3) => Format::Rgb8Sint,
                (Format::R8Sint, 4) => Format::Rgba8Sint,
                (Format::R16Sint, 2) => Format::Rg16Sint,
                (Format::R16Sint, 3) => Format::Rgb16Sint,
                (Format::R16Sint, 4) => Format::Rgba16Sint,
                (Format::R32Sint, 2) => Format::Rg32Sint,
                (Format::R32Sint, 3) => Format::Rgb32Sint,
                (Format::R32Sint, 4) => Format::Rgba32Sint,
                (Format::R64Sint, 2) => Format::Rg64Sint,
                (Format::R64Sint, 3) => Format::Rgb64Sint,
                (Format::R64Sint, 4) => Format::Rgba64Sint,
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
                (Format::R32Sfloat, 2) => Format::Rg32Sfloat,
                (Format::R32Sfloat, 3) => Format::Rgb32Sfloat,
                (Format::R32Sfloat, 4) => Format::Rgba32Sfloat,
                (Format::R64Sfloat, 2) => Format::Rg64Sfloat,
                (Format::R64Sfloat, 3) => Format::Rgb64Sfloat,
                (Format::R64Sfloat, 4) => Format::Rgba64Sfloat,
                (format, count) => {
                    return Err(ReflectTypeError::UnrecognizedNumericArrayCount(
                        format, count,
                    ))
                }
            },
        )
    } else {
        Ok(current_type)
    }
}

impl ReflectInto<rendy_core::hal::pso::Element<Format>> for ReflectTypeDescription {
    fn reflect_into(&self) -> Result<rendy_core::hal::pso::Element<Format>, ReflectTypeError> {
        let format = type_element_format(self.type_flags, &self.traits)?;
        Ok(rendy_core::hal::pso::Element {
            format: format,
            offset: 0,
        })
    }
}

impl ReflectInto<rendy_core::hal::pso::AttributeDesc> for ReflectInterfaceVariable {
    fn reflect_into(&self) -> Result<rendy_core::hal::pso::AttributeDesc, ReflectTypeError> {
        // An attribute is not an image format
        Ok(rendy_core::hal::pso::AttributeDesc {
            location: self.location,
            binding: self.location,
            element: self
                .type_description
                .as_ref()
                .ok_or(ReflectTypeError::VertexElement)
                .and_then(ReflectInto::reflect_into)?,
        })
    }
}

// Descriptor Sets
//

impl ReflectInto<rendy_core::hal::pso::DescriptorType> for ReflectDescriptorType {
    fn reflect_into(&self) -> Result<rendy_core::hal::pso::DescriptorType, ReflectTypeError> {
        use rendy_core::hal::pso::DescriptorType;
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
            AccelerationStructureNV => Err(ReflectTypeError::UnhandledAccelerationStructureNV),
            Undefined => Err(ReflectTypeError::UnhandledUndefined),
        }
    }
}

impl ReflectInto<Vec<rendy_core::hal::pso::DescriptorSetLayoutBinding>> for ReflectDescriptorSet {
    fn reflect_into(
        &self,
    ) -> Result<Vec<rendy_core::hal::pso::DescriptorSetLayoutBinding>, ReflectTypeError> {
        self.bindings
            .iter()
            .map(|desc| desc.reflect_into())
            .collect::<Result<Vec<_>, _>>()
    }
}

impl ReflectInto<rendy_core::hal::pso::DescriptorSetLayoutBinding> for ReflectDescriptorBinding {
    fn reflect_into(
        &self,
    ) -> Result<rendy_core::hal::pso::DescriptorSetLayoutBinding, ReflectTypeError> {
        Ok(rendy_core::hal::pso::DescriptorSetLayoutBinding {
            binding: self.binding,
            ty: self.descriptor_type.reflect_into()?,
            count: self.count as usize,
            stage_flags: rendy_core::hal::pso::ShaderStageFlags::VERTEX,
            immutable_samplers: false, // TODO: how to determine this?
        })
    }
}

pub(crate) fn convert_push_constant(
    stage: rendy_core::hal::pso::ShaderStageFlags,
    variable: &ReflectBlockVariable,
) -> Result<(rendy_core::hal::pso::ShaderStageFlags, std::ops::Range<u32>), ReflectTypeError> {
    Ok((
        stage,
        variable.offset..variable.offset / 4 + variable.size / 4,
    ))
}

pub(crate) fn convert_stage(
    stage: ReflectShaderStageFlags,
) -> rendy_core::hal::pso::ShaderStageFlags {
    let mut bits = rendy_core::hal::pso::ShaderStageFlags::empty();

    if stage.contains(ReflectShaderStageFlags::VERTEX) {
        bits |= rendy_core::hal::pso::ShaderStageFlags::VERTEX;
    }
    if stage.contains(ReflectShaderStageFlags::FRAGMENT) {
        bits |= rendy_core::hal::pso::ShaderStageFlags::FRAGMENT;
    }
    if stage.contains(ReflectShaderStageFlags::GEOMETRY) {
        bits |= rendy_core::hal::pso::ShaderStageFlags::GEOMETRY;
    }
    if stage.contains(ReflectShaderStageFlags::COMPUTE) {
        bits |= rendy_core::hal::pso::ShaderStageFlags::COMPUTE;
    }
    if stage.contains(ReflectShaderStageFlags::TESSELLATION_CONTROL) {
        bits |= rendy_core::hal::pso::ShaderStageFlags::HULL;
    }
    if stage.contains(ReflectShaderStageFlags::TESSELLATION_EVALUATION) {
        bits |= rendy_core::hal::pso::ShaderStageFlags::DOMAIN;
    }

    bits
}

pub(crate) fn generate_attributes(
    attributes: Vec<ReflectInterfaceVariable>,
) -> Result<HashMap<(String, u8), rendy_core::hal::pso::AttributeDesc>, ReflectTypeError> {
    let mut out_attributes = HashMap::new();

    for attribute in &attributes {
        if attribute
            .decoration_flags
            .contains(ReflectDecorationFlags::BUILT_IN)
        {
            continue;
        }

        let reflected: rendy_core::hal::pso::AttributeDesc = attribute.reflect_into()?;
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
