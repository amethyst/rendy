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
pub struct FileShaderInfo<P, E> {
    path: P,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<P, E> FileShaderInfo<P, E> {
    /// New shader.
    pub fn new(path: P, kind: ShaderKind, lang: SourceLanguage, entry: E) -> Self {
        FileShaderInfo {
            path,
            kind,
            lang,
            entry,
        }
    }
}

impl<P, E> FileShaderInfo<P, E>
where
    E: AsRef<str>,
{
    /// Precompile shader source code into Spir-V bytecode.
    pub fn precompile(&self) -> Result<SpirvShader, failure::Error>
    where
        Self: Shader,
    {
        Ok(SpirvShader::new(
            self.spirv()?.into_owned(),
            stage_from_kind(&self.kind),
            self.entry.as_ref(),
        ))
    }
}

impl<P, E> Shader for FileShaderInfo<P, E>
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
                    ops.set_generate_debug_info();
                    ops.set_optimization_level(shaderc::OptimizationLevel::Performance);
                    ops
                })
                .as_ref(),
            )?;

        Ok(std::borrow::Cow::Owned(artifact.as_binary_u8().into()))
    }

    fn entry(&self) -> &str {
        self.entry.as_ref()
    }

    fn stage(&self) -> gfx_hal::pso::ShaderStageFlags {
        stage_from_kind(&self.kind)
    }
}


/// Shader loaded from a source in the filesystem.
#[derive(Clone, Copy, Debug)]
pub struct SourceCodeShaderInfo<P, E, S> {
    source: S,
    path: P,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<P, E, S> SourceCodeShaderInfo<P, E, S> {
    /// New shader.
    pub fn new(source: S, path: P, kind: ShaderKind, lang: SourceLanguage, entry: E) -> Self {
        SourceCodeShaderInfo {
            source,
            path,
            kind,
            lang,
            entry,
        }
    }
}

impl<P, E, S> SourceCodeShaderInfo<P, E, S>
    where
        E: AsRef<str>,
{
    /// Precompile shader source code into Spir-V bytecode.
    pub fn precompile(&self) -> Result<SpirvShader, failure::Error>
        where
            Self: Shader,
    {
        Ok(SpirvShader::new(
            self.spirv()?.into_owned(),
            stage_from_kind(&self.kind),
            self.entry.as_ref(),
        ))
    }
}

impl<P, E, S> Shader for SourceCodeShaderInfo<P, E, S>
    where
        P: AsRef<std::path::Path> + std::fmt::Debug,
        E: AsRef<str>,
        S: AsRef<str> + std::fmt::Debug,
{
    fn spirv(&self) -> Result<std::borrow::Cow<'static, [u8]>, failure::Error> {
        let artifact = shaderc::Compiler::new()
            .ok_or_else(|| failure::format_err!("Failed to init Shaderc"))?
            .compile_into_spirv(
                self.source.as_ref(),
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
                    ops.set_generate_debug_info();
                    ops.set_optimization_level(shaderc::OptimizationLevel::Performance);
                    ops
                })
                    .as_ref(),
            )?;

        Ok(std::borrow::Cow::Owned(artifact.as_binary_u8().into()))
    }

    fn entry(&self) -> &str {
        self.entry.as_ref()
    }

    fn stage(&self) -> gfx_hal::pso::ShaderStageFlags {
        stage_from_kind(&self.kind)
    }
}

/// Shader info with static data.
pub type SourceShaderInfo = SourceCodeShaderInfo<&'static str, &'static str, &'static str>;

/// DEPRECATED. USE `PathBufShaderInfo` INSTEAD!
#[deprecated(since = "2.0", note = "StaticShaderInfo will be removed in favor of PathBufShaderInfo soon. Please move to that implementation.")]
pub type StaticShaderInfo = FileShaderInfo<&'static str, &'static str>;

/// Shader info with a PathBuf for the path and static string for entry
pub type PathBufShaderInfo = FileShaderInfo<std::path::PathBuf, &'static str>;

fn stage_from_kind(kind: &ShaderKind) -> gfx_hal::pso::ShaderStageFlags {
    use gfx_hal::pso::ShaderStageFlags;
    match kind {
        ShaderKind::Vertex => ShaderStageFlags::VERTEX,
        ShaderKind::Fragment => ShaderStageFlags::FRAGMENT,
        ShaderKind::Geometry => ShaderStageFlags::GEOMETRY,
        ShaderKind::TessEvaluation => ShaderStageFlags::HULL,
        ShaderKind::TessControl => ShaderStageFlags::DOMAIN,
        ShaderKind::Compute => ShaderStageFlags::COMPUTE,
        _ => panic!("Invalid shader type specified"),
    }
}
