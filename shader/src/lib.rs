//! Shader compilation.

#[cfg(feature = "shader-compiler")]
mod shaderc;
#[cfg(feature = "spirv-reflection")]
pub mod reflect;

#[cfg(feature = "shader-compiler")]
pub use self::shaderc::*;
#[cfg(feature = "spirv-reflection")]
pub use self::reflect::*;

#[warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

/// Interface to create shader modules from shaders.
/// Implemented for static shaders via [`compile_to_spirv!`] macro.
///
pub trait Shader {
    /// Get spirv bytecode.
    fn spirv(&self) -> Result<std::borrow::Cow<'_, [u8]>, failure::Error>;

    #[cfg(feature = "spirv-reflection")]
    /// Uses spirv-reflect to generate a [SpirvShaderDescription] reflection representation, which is
    /// an intermediate to gfx_hal data representations.
    fn reflect(&self) -> Result<SpirvShaderDescription, failure::Error> {
        Ok(reflect::SpirvShaderDescription::from_bytes(
            &*(self.spirv()?),
        )?)
    }

    /// Create shader module.
    fn module<B>(
        &self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<B::ShaderModule, failure::Error>
    where
        B: gfx_hal::Backend,
    {
        unsafe { gfx_hal::Device::create_shader_module(factory.device(), &self.spirv()?) }
            .map_err(Into::into)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpirvShaderInfo {
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    spirv: Vec<u8>,
}

impl SpirvShaderInfo {
    pub fn new(spirv: Vec<u8>) -> Self {
        Self { spirv }
    }
}

impl Shader for SpirvShaderInfo {
    fn spirv(&self) -> Result<std::borrow::Cow<'_, [u8]>, failure::Error> {
        Ok(std::borrow::Cow::Borrowed(&self.spirv))
    }
}
