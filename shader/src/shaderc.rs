// This module is gated under "shader-compiler" feature
use super::Shader;
use crate::SpirvShader;
pub use shaderc::{self, ShaderKind, SourceLanguage};

macro_rules! vk_make_version {
    ($major: expr, $minor: expr, $patch: expr) => {{
        let (major, minor, patch): (u32, u32, u32) = ($major, $minor, $patch);
        (major << 22) | (minor << 12) | patch
    }};
}

/// Shader loaded from a source in the filesystem.
#[derive(Clone, Copy, Debug)]
pub struct SourceShaderInfo<P, E> {
    path: P,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<P, E> SourceShaderInfo<P, E> {
    /// New shader.
    pub fn new(path: P, kind: ShaderKind, lang: SourceLanguage, entry: E) -> Self {
        SourceShaderInfo {
            path,
            kind,
            lang,
            entry,
        }
    }
}

impl<P, E> SourceShaderInfo<P, E> {
    /// Precompile shader source code into Spir-V bytecode.
    pub fn precompile(&self) -> Result<SpirvShader, failure::Error>
    where
        Self: Shader,
    {
        Ok(SpirvShader::new(self.spirv()?.into_owned()))
    }
}

impl<P, E> Shader for SourceShaderInfo<P, E>
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
                self.path.as_ref().to_str().ok_or_else(|| {
                    failure::format_err!("'{:?}' is not valid UTF-8 string", self.path)
                })?,
                self.entry.as_ref(),
                Some({
                    let mut ops = shaderc::CompileOptions::new()
                        .ok_or_else(|| failure::format_err!("Failed to init Shaderc"))?;
                    ops.set_target_env(shaderc::TargetEnv::Vulkan, vk_make_version!(1, 0, 0));
                    ops.set_source_language(self.lang);
                    //ops.set_generate_debug_info();
                    //ops.set_optimization_level(shaderc::OptimizationLevel::None);
                    ops.set_optimization_level(shaderc::OptimizationLevel::Performance);;
                    ops
                })
                .as_ref(),
            )?;

        Ok(std::borrow::Cow::Owned(artifact.as_binary_u8().into()))
    }
}

/// Shader info with static data.
pub type StaticShaderInfo = SourceShaderInfo<&'static str, &'static str>;
