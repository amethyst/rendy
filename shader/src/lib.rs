
//! Shader compilation.

#[warn(missing_debug_implementations,
       missing_copy_implementations,
       missing_docs,
       trivial_casts,
       trivial_numeric_casts,
       unused_extern_crates,
       unused_import_braces,
       unused_qualifications)]

pub use shaderc::{self, ShaderKind, SourceLanguage};

macro_rules! vk_make_version {
    ($major: expr, $minor: expr, $patch: expr) => ((($major as u32) << 22) | (($minor as u32) << 12) | $patch as u32)
}

/// Interface to create shader modules from shaders.
/// Implemented for static shaders via [`compile_to_spirv!`] macro.
/// 
pub trait Shader {
    /// Get spirv bytecode.
    fn spirv(&self) -> Result<std::borrow::Cow<'static, [u8]>, failure::Error>;

    /// Create shader module.
    fn module<B>(&self, factory: &rendy_factory::Factory<B>) -> Result<B::ShaderModule, failure::Error>
    where
        B: gfx_hal::Backend,
    {
        unsafe {
            gfx_hal::Device::create_shader_module(factory.device(), &self.spirv()?)
        }.map_err(Into::into)
    }
}

/// Dynamic shader.
#[derive(Clone, Copy, Debug)]
pub struct ShaderInfo<P, E> {
    path: P,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<P, E> ShaderInfo<P, E> {

    /// New dynamic shader.
    pub fn new(path: P, kind: ShaderKind, lang: SourceLanguage, entry: E) -> Self {
        ShaderInfo {
            path: path.into(),
            kind,
            lang,
            entry: entry.into(),
        }
    }
}

impl<P, E> Shader for ShaderInfo<P, E>
where
    P: AsRef<std::path::Path> + std::fmt::Debug,
    E: AsRef<str>,
{
    fn spirv(&self) -> Result<std::borrow::Cow<'static, [u8]>, failure::Error> {
        let code = std::fs::read_to_string(&self.path)?;

        let artifact = shaderc::Compiler::new()
            .ok_or_else(|| failure::format_err!("Failed to init Shaderc"))?
            .compile_into_spirv(
                &code,
                self.kind,
                self.path.as_ref().to_str().ok_or_else(|| failure::format_err!("'{:?}' is not valid UTF-8 string", self.path))?,
                self.entry.as_ref(),
                Some({
                    let mut ops = shaderc::CompileOptions::new().ok_or_else(|| failure::format_err!("Failed to init Shaderc"))?;
                    ops.set_target_env(shaderc::TargetEnv::Vulkan, vk_make_version!(1, 0, 0));
                    ops.set_source_language(self.lang);
                    ops.set_optimization_level(shaderc::OptimizationLevel::Performance);
                    ops
                }).as_ref(),
            )?;

        Ok(std::borrow::Cow::Owned(artifact.as_binary_u8().into()))
    }
}

/// Shader info with static data.
pub type StaticShaderInfo = ShaderInfo<&'static str, &'static str>;
