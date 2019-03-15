// This module is gated under "shader-compiler" feature
use super::Shader;
pub use shaderc::{self, ShaderKind, SourceLanguage};

#[cfg(feature = "spirv-reflection")]
use crate::reflect;

macro_rules! vk_make_version {
    ($major: expr, $minor: expr, $patch: expr) => {
        (($major as u32) << 22) | (($minor as u32) << 12) | $patch as u32
    };
}

/// Shader loaded from a source in the filesystem.
#[derive(Clone, Copy, Debug)]
pub struct SourceShaderInfo<P, E> {
    path: P,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<P, E> SourceShaderInfo<P, E>
    where
        P: AsRef<std::path::Path> + std::fmt::Debug,
        E: AsRef<str>,
{
    /// New shader.
    pub fn new(path: P, kind: ShaderKind, lang: SourceLanguage, entry: E) -> Self {
        SourceShaderInfo {
            path,
            kind,
            lang,
            entry,
        }
    }

    fn compile(&self, debug: bool) -> Result<std::borrow::Cow<'static, [u8]>, failure::Error> {
        let code = std::fs::read_to_string(&self.path)?;

        let artifact = shaderc::Compiler::new()
            .ok_or_else(|| failure::format_err!("Failed to init Shaderc"))?
            .compile_into_spirv(
                &code,
                self.kind,
                self.path.as_ref().to_str().ok_or_else(|| {
                    failure::format_err!("'{:?}' is not valid UTF-8 string", self.path)
                })?,
                self.entry.as_ref(),
                Some({
                    let mut ops = shaderc::CompileOptions::new()
                        .ok_or_else(|| failure::format_err!("Failed to init Shaderc"))?;
                    ops.set_target_env(shaderc::TargetEnv::Vulkan, vk_make_version!(1, 0, 0));
                    ops.set_source_language(self.lang);

                    if debug {
                        ops.set_optimization_level(shaderc::OptimizationLevel::Zero);
                        ops.set_generate_debug_info();
                    } else {
                        ops.set_optimization_level(shaderc::OptimizationLevel::Performance);
                    }
                    ops
                })
                    .as_ref(),
            )?;

        Ok(std::borrow::Cow::Owned(artifact.as_binary_u8().into()))
    }
}

impl<P, E> Shader for SourceShaderInfo<P, E>
where
    P: AsRef<std::path::Path> + std::fmt::Debug,
    E: AsRef<str>,
{
    fn spirv(&self) -> Result<std::borrow::Cow<'static, [u8]>, failure::Error> {
        self.compile(false)
    }

    #[cfg(feature = "spirv-reflection")]
    fn reflect(&self) -> Result<reflect::SpirvShaderDescription, failure::Error> {
        Ok(reflect::SpirvShaderDescription::from_bytes(&*(self.compile(true)?), true)?)
    }
}

/// Shader info with static data.
pub type StaticShaderInfo = SourceShaderInfo<&'static str, &'static str>;
