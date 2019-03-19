/// Reflection extensions

use rendy_shader::reflect::SpirvShaderDescription;
use crate::node::render::{Layout, SetLayout};

/// Extension for SpirvShaderReflection providing graph render type conversion
pub trait ShaderLayoutGenerator {
    /// Convert reflected descriptor sets to a Layout structure
    fn layout(&self) -> Layout;

    /// Convert reflected attributes to a direct gfx_hal element array
    fn attributes(&self) -> (Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>, gfx_hal::pso::ElemStride);
}

impl ShaderLayoutGenerator for SpirvShaderDescription {
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
        let elements: Vec<gfx_hal::pso::Element<gfx_hal::format::Format>> = self.input_attributes.iter()
            .filter(|(k, _)|{ !k.is_empty() })
            .map(|(_, v)| { v.element } ).collect();

        (elements, stride)
    }
}

impl ShaderLayoutGenerator for (SpirvShaderDescription, SpirvShaderDescription) {
    fn layout(&self) -> Layout {
        use rendy_shader::reflect::AsVector;

        let mut sets = self.0.descriptor_sets.iter().map(|set| SetLayout { bindings: set.as_vector() }).collect::<Vec<_>>();
        sets.append(&mut self.1.descriptor_sets.iter().map(|set| SetLayout { bindings: set.as_vector() }).collect::<Vec<_>>());

        Layout {
            sets,
            push_constants: Vec::new(),
        }
    }

    fn attributes(&self) -> (Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>, gfx_hal::pso::ElemStride)
    {
        let stride: u32 = 0;
        let elements: Vec<gfx_hal::pso::Element<gfx_hal::format::Format>> = self.0.input_attributes.iter()
            .filter(|(k, _)|{ !k.is_empty() })
            .map(|(_, v)| { v.element } ).collect();

        (elements, stride)
    }
}