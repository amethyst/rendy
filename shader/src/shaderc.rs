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
#[derive(Clone, Debug)]
pub struct SourceShaderInfo<E> {
    source_text: String,
    input_file_name: String,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<E> SourceShaderInfo<E> {
    /// New shader loaded from a file on the filesystem.
    pub fn new<P>(path: P, kind: ShaderKind, lang: SourceLanguage, entry: E) -> Result<Self, failure::Error>
        where P: AsRef<std::path::Path> + std::fmt::Debug, {
        let input_file_name = path.as_ref().to_str().ok_or_else(|| {
            failure::format_err!("'{:?}' is not valid UTF-8 string", path)
        })?
        .to_string();
        let source_text = std::fs::read_to_string(&input_file_name)?;
        Ok(SourceShaderInfo {
            source_text,
            input_file_name,
            kind,
            lang,
            entry,
        })
    }

    /// New shader loaded from a string.
    ///
    /// The program name is used for referring to this particular
    /// program in debugging output.
    pub fn new_from_str(source: &str, program_name: &str, kind: ShaderKind, lang: SourceLanguage, entry: E) -> Self {
        let input_file_name = program_name.to_owned();
        let source_text = source.to_owned();
        SourceShaderInfo {
            source_text,
            input_file_name,
            kind,
            lang,
            entry,
        }
    }
}

impl<E> SourceShaderInfo<E>
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

impl<E> Shader for SourceShaderInfo<E>
where
    E: AsRef<str>,
{
    fn spirv(&self) -> Result<std::borrow::Cow<'static, [u8]>, failure::Error> {
        let artifact = shaderc::Compiler::new()
            .ok_or_else(|| failure::format_err!("Failed to init Shaderc"))?
            .compile_into_spirv(
                &self.source_text,
                self.kind,
                self.input_file_name.as_ref(),
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
pub type StaticShaderInfo = SourceShaderInfo<&'static str>;

/// Shader info with a PathBuf for the path and static string for entry
pub type PathBufShaderInfo = SourceShaderInfo<&'static str>;

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
