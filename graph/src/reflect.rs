/// Reflection extensions

use rendy_shader::reflect::SpirvShaderDescription;
use crate::node::render::{Layout, SetLayout};

/// Extension for SpirvShaderReflection providing graph render type conversion
pub trait ShaderReflectBuilder {
    /// Convert reflected descriptor sets to a Layout structure
    fn layout(&self) -> Layout;

    /// Convert reflected attributes to a direct gfx_hal element array
    fn attributes(&self) -> (Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>, gfx_hal::pso::ElemStride);
}

impl ShaderReflectBuilder for SpirvShaderDescription {
    fn layout(&self) -> Layout {
        use rendy_shader::reflect::AsVector;

        Layout {
            sets: self.descriptor_sets.iter().map(|set| SetLayout { bindings: set.as_vector() }).collect(),
            push_constants: Vec::new(),
        }
    }

    fn attributes(&self) -> (Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>, gfx_hal::pso::ElemStride)
    {
        let stride: u32 = 0;
        let elements: Vec<gfx_hal::pso::Element<gfx_hal::format::Format>> = self.input_variables.iter()
            .filter(|(k, _)|{
                if k.contains("gl_") || k.is_empty() {
                    return false
                }
                true
            })
            .map(|(_, v)| {
                v.element
            } ).collect();

        (elements, stride)
    }
}