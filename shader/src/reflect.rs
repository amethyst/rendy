//! Using spirv-reflect-rs for reflection.
//!

use log::{trace};

use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

use spirv_reflect::{
    ShaderModule,
    types::*,
};

use gfx_hal::format::Format;

/// Workaround extension trait copy of std::convert::From, for simple conversion from spirv-reflect types to gfx_hal types
pub trait ReflectInto<T>: Sized {
    /// Attempts to perform a conversion from the provided type into this type
    fn reflect_into(&self) -> Result<T, failure::Error> {
        Err(failure::format_err!("Unsupported conversion type"))
    }
}

/// Harness type for easier conversions of named return collections.
pub trait AsVector<V> {
    /// Implemented to return a straight vector from a hashmap, so the user doesnt have to map.collect for all its uses
    /// This function clones all values in the hashmap so beware.
    fn as_vector(&self, ) -> Vec<V>;
}

impl<K, V> AsVector<V> for HashMap<K, V>
    where
        K: Eq + std::hash::Hash,
        V: Sized + Clone,
{
    fn as_vector(&self, ) -> Vec<V> {
        self.into_iter().map(|(_, i)| { (*i).clone() }).collect()
    }
}

impl ReflectInto<Format> for image::ReflectFormat {
    fn reflect_into(&self, ) -> Result<Format, failure::Error> {
        match self {
            image::ReflectFormat::Undefined => Err(failure::format_err!("Undefined Format")),
            image::ReflectFormat::R32_UINT => Ok(Format::R32Uint),
            image::ReflectFormat::R32_SINT => Ok(Format::R32Int),
            image::ReflectFormat::R32_SFLOAT => Ok(Format::R32Float),
            image::ReflectFormat::R32G32_UINT => Ok(Format::Rg32Uint),
            image::ReflectFormat::R32G32_SINT => Ok(Format::Rg32Int),
            image::ReflectFormat::R32G32_SFLOAT => Ok(Format::Rg32Float),
            image::ReflectFormat::R32G32B32_UINT => Ok(Format::Rgb32Uint),
            image::ReflectFormat::R32G32B32_SINT => Ok(Format::Rgb32Int),
            image::ReflectFormat::R32G32B32_SFLOAT => Ok(Format::Rgb32Float),
            image::ReflectFormat::R32G32B32A32_UINT => Ok(Format::Rgb32Uint),
            image::ReflectFormat::R32G32B32A32_SINT => Ok(Format::Rgb32Int),
            image::ReflectFormat::R32G32B32A32_SFLOAT => Ok(Format::Rgb32Float),
        }
    }
}

fn type_element_format(flags: variable::ReflectTypeFlags, traits: &traits::ReflectTypeDescriptionTraits) -> Result<gfx_hal::format::Format, failure::Error> {
    let mut current_type = Format::R32Float;

    if flags.contains(variable::ReflectTypeFlags::INT) {
        current_type = match traits.numeric.scalar.signedness {
            1 => match traits.numeric.scalar.width {
                8 => Format::R8Int,
                16 => Format::R16Int,
                32 => Format::R32Int,
                64 => Format::R64Int,
                _ => return Err(failure::format_err!("Unrecognized scalar width for int")),
            },
            0 => match traits.numeric.scalar.width {
                8 => Format::R8Uint,
                16 => Format::R16Uint,
                32 => Format::R32Uint,
                64 => Format::R64Uint,
                _ => return Err(failure::format_err!("Unrecognized scalar width for unsigned int")),
            },
            _ => return Err(failure::format_err!("Invalid signedness flag")),
        };
    }
    if flags.contains(variable::ReflectTypeFlags::FLOAT) {
        // TODO: support other bits
        current_type = match traits.numeric.scalar.width {
            32 => Format::R32Float,
            64 => Format::R64Float,
            _ => return Err(failure::format_err!("Unrecognized scalar width for float")),
        }
    }

    if flags.contains(variable::ReflectTypeFlags::VECTOR) {
        current_type = match traits.numeric.vector.component_count {
            2 => match current_type {
                Format::R64Float => Format::Rg64Float,
                Format::R32Float => Format::Rg32Float,
                Format::R32Int => Format::Rg32Int,
                Format::R32Uint => Format::Rg32Int,
                _ => return Err(failure::format_err!("Unknown type for vector: {:?}", current_type)),
            },
            3 => match current_type {
                Format::R64Float => Format::Rgb64Float,
                Format::R32Float => Format::Rgb32Float,
                Format::R32Int => Format::Rgb32Int,
                Format::R32Uint => Format::Rgb32Int,
                _ => return Err(failure::format_err!("Unknown type for vector: {:?}", current_type)),
            },
            4 => match current_type {
                Format::R64Float => Format::Rgba64Float,
                Format::R32Float => Format::Rgba32Float,
                Format::R32Int => Format::Rgba32Int,
                Format::R32Uint => Format::Rgba32Int,
                _ => return Err(failure::format_err!("Unknown type for vector: {:?}", current_type)),
            },
            _ => return Err(failure::format_err!("Invalid vector size: {:?}", traits.numeric.vector.component_count)),
        };
    }

    Ok(current_type)
}

impl ReflectInto<gfx_hal::pso::Element<gfx_hal::format::Format>> for variable::ReflectTypeDescription {
    fn reflect_into(&self, ) -> Result<gfx_hal::pso::Element<gfx_hal::format::Format>, failure::Error> {
        Ok(gfx_hal::pso::Element { format: type_element_format(self.type_flags, &self.traits)?, offset: 0, })
    }
}

impl ReflectInto<(String, gfx_hal::pso::AttributeDesc)> for variable::ReflectInterfaceVariable {
    fn reflect_into(&self) -> Result<(String, gfx_hal::pso::AttributeDesc), failure::Error> {
        // An attribute is not an image format
        Ok((self.name.clone(), gfx_hal::pso::AttributeDesc {
            location: self.location,
            binding: self.location,
            element: self.type_description.as_ref()
                .ok_or_else(||failure::format_err!("Unable to reflect vertex element"))?.reflect_into()?,
        }))
    }
}



// Descriptor Sets
//


impl ReflectInto<gfx_hal::pso::DescriptorType> for descriptor::ReflectDescriptorType {
    fn reflect_into(&self, ) -> Result<gfx_hal::pso::DescriptorType, failure::Error> {
        match *self {
            descriptor::ReflectDescriptorType::Sampler => Ok(gfx_hal::pso::DescriptorType::Sampler),
            descriptor::ReflectDescriptorType::CombinedImageSampler => Ok(gfx_hal::pso::DescriptorType::CombinedImageSampler),
            descriptor::ReflectDescriptorType::SampledImage => Ok(gfx_hal::pso::DescriptorType::SampledImage),
            descriptor::ReflectDescriptorType::StorageImage => Ok(gfx_hal::pso::DescriptorType::StorageImage),
            descriptor::ReflectDescriptorType::UniformTexelBuffer => Ok(gfx_hal::pso::DescriptorType::UniformTexelBuffer),
            descriptor::ReflectDescriptorType::StorageTexelBuffer => Ok(gfx_hal::pso::DescriptorType::StorageTexelBuffer),
            descriptor::ReflectDescriptorType::UniformBuffer => Ok(gfx_hal::pso::DescriptorType::UniformBuffer),
            descriptor::ReflectDescriptorType::StorageBuffer => Ok(gfx_hal::pso::DescriptorType::StorageBuffer),
            descriptor::ReflectDescriptorType::UniformBufferDynamic => Ok(gfx_hal::pso::DescriptorType::UniformBufferDynamic),
            descriptor::ReflectDescriptorType::StorageBufferDynamic => Ok(gfx_hal::pso::DescriptorType::StorageBufferDynamic),
            descriptor::ReflectDescriptorType::InputAttachment => Ok(gfx_hal::pso::DescriptorType::InputAttachment),
            descriptor::ReflectDescriptorType::AccelerationStructureNV => Err(failure::format_err!("We cant handle AccelerationStructureNV descriptor type")),
            descriptor::ReflectDescriptorType::Undefined => Err(failure::format_err!("We cant handle undefined descriptor types")),
        }
    }
}

impl ReflectInto<HashMap<String, gfx_hal::pso::DescriptorSetLayoutBinding>> for descriptor::ReflectDescriptorSet {
    fn reflect_into(&self, ) -> Result<HashMap<String, gfx_hal::pso::DescriptorSetLayoutBinding>, failure::Error> {
        let mut output = HashMap::<String, gfx_hal::pso::DescriptorSetLayoutBinding>::new();

        for descriptor in self.bindings.iter() {
            assert!(!output.contains_key(&descriptor.name));
            output.insert(descriptor.name.clone(), descriptor.reflect_into()?);
        }

        Ok(output)
    }
}
impl ReflectInto<gfx_hal::pso::DescriptorSetLayoutBinding> for descriptor::ReflectDescriptorBinding {
    fn reflect_into(&self, ) -> Result<gfx_hal::pso::DescriptorSetLayoutBinding, failure::Error> {
        Ok(gfx_hal::pso::DescriptorSetLayoutBinding {
            binding: self.binding,
            ty: self.descriptor_type.reflect_into()?,
            count: self.count as usize,
            stage_flags: gfx_hal::pso::ShaderStageFlags::VERTEX,
            immutable_samplers: false, // TODO: how to determine this?
        })
    }
}

fn convert_stage(stage: variable::ReflectShaderStageFlags) -> gfx_hal::pso::ShaderStageFlags {
    let mut bits = gfx_hal::pso::ShaderStageFlags::empty();

    if stage.contains(variable::ReflectShaderStageFlags::VERTEX) {
        bits |= gfx_hal::pso::ShaderStageFlags::VERTEX;
    }
    if stage.contains(variable::ReflectShaderStageFlags::FRAGMENT) {
        bits |= gfx_hal::pso::ShaderStageFlags::FRAGMENT;
    }
    if stage.contains(variable::ReflectShaderStageFlags::GEOMETRY) {
        bits |= gfx_hal::pso::ShaderStageFlags::GEOMETRY;
    }
    if stage.contains(variable::ReflectShaderStageFlags::COMPUTE) {
        bits |= gfx_hal::pso::ShaderStageFlags::COMPUTE;
    }
    if stage.contains(variable::ReflectShaderStageFlags::TESSELLATION_CONTROL) {
        bits |= gfx_hal::pso::ShaderStageFlags::HULL;
    }
    if stage.contains(variable::ReflectShaderStageFlags::TESSELLATION_EVALUATION) {
        bits |= gfx_hal::pso::ShaderStageFlags::DOMAIN;
    }

    bits
}

/// Implementation of shader reflection for SPIRV
#[derive(Clone)]
pub struct SpirvShaderDescription {
    /// Hashmap of output variables with names.
    pub output_variables: HashMap<String, gfx_hal::pso::AttributeDesc>,
    /// Hashmap of output variables with names.
    pub input_variables: HashMap<String, gfx_hal::pso::AttributeDesc>,
    /// Hashmap of output variables with names.
    pub descriptor_sets: Vec<HashMap<String, gfx_hal::pso::DescriptorSetLayoutBinding>>,
}

impl SpirvShaderDescription {
    ///
    pub fn from_bytes(data: &[u8], strict: bool) -> Result<Self, failure::Error> {
        trace!("Shader reflecting into SpirvShaderDescription");
        match ShaderModule::load_u8_data(data) {
            Ok(module) => {
                Ok(Self{
                    // TODO: change these unwraps back to actual error checking
                    // TODO: strict isnt really strict
                    input_variables: module.enumerate_input_variables(None).map_err(|_| failure::format_err!("Cant get input variables") )?.iter()
                        .filter(|v| {
                            let r = v.reflect_into();
                            match strict {
                                true => { r.is_ok() && !r.as_ref().unwrap().0.is_empty() },
                                false => r.is_ok(),
                            }
                        })
                        .map(|v| {
                            v.reflect_into().unwrap()
                        })
                        .collect(),
                    output_variables: module.enumerate_output_variables(None).map_err(|_| failure::format_err!("Cant get output variables") )?.iter()
                        .filter(|v| {
                            let r = v.reflect_into();
                            match strict {
                                true => { r.is_ok() && !r.as_ref().unwrap().0.is_empty() },
                                false => r.is_ok(),
                            }
                        })
                        .map(|v| {
                            v.reflect_into().unwrap()
                        }).
                        collect(),
                    descriptor_sets: module.enumerate_descriptor_sets(None).map_err(|_| failure::format_err!("Cant get descriptor sets") )?.iter()
                        .filter(|v| {
                            match strict {
                                true => { v.reflect_into().unwrap(); true },
                                false => v.reflect_into().is_ok(),
                            }
                        })
                        .map(|v| {
                            let mut values = v.reflect_into().unwrap();
                            // Fix shader stages
                            values.iter_mut().for_each(|(_, set)| {
                                set.stage_flags = convert_stage(module.get_shader_stage());
                            });
                            values
                        })
                        .collect(),
                })
            },
            Err(_) => {
                Err(failure::format_err!("Failed to load module data"))
            }
        }
    }

    ///
    pub fn from_file<P>(path: P, strict: bool) -> Result<Self, failure::Error>
        where
            P: AsRef<std::path::Path> + std::fmt::Debug,
    {
        let mut file = File::open(path)?;
        let mut contents: Vec<u8> = Vec::with_capacity(file.metadata()?.len() as usize);
        file.read_to_end(&mut contents)?;

        Self::from_bytes(contents.as_slice(), strict)
    }
}

impl std::fmt::Debug for SpirvShaderDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for input in &self.input_variables {
            write!(f, "input: {:?}\n", input)?;
        }

        for output in &self.output_variables {
            write!(f, "output: {:?}\n", output)?;
        }

        for output in &self.descriptor_sets {
            write!(f, "descriptors: {:?}\n", output)?;
        }
        Ok(())
    }
}