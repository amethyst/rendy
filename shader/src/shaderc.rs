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

/// Error type returned by shader compiler functionality.
#[derive(Debug)]
pub enum ShaderCError {
    /// Shaderc could not be initialized.
    Init,
    /// The given path is not a valid UTF-8 string.
    NonUtf8Path(std::path::PathBuf),
    /// An io error occured.
    Io(std::io::Error),
    /// Shaderc returned an error.
    ShaderC(::shaderc::Error),
}

impl std::error::Error for ShaderCError {}
impl std::fmt::Display for ShaderCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderCError::Init => write!(f, "failed to init Shaderc"),
            ShaderCError::NonUtf8Path(path) => {
                write!(f, "path {:?} is not valid UTF-8 string", path)
            }
            ShaderCError::Io(e) => write!(f, "{}", e),
            ShaderCError::ShaderC(e) => write!(f, "{}", e),
        }
    }
}

impl From<std::io::Error> for ShaderCError {
    fn from(e: std::io::Error) -> Self {
        ShaderCError::Io(e)
    }
}

impl From<::shaderc::Error> for ShaderCError {
    fn from(e: ::shaderc::Error) -> Self {
        ShaderCError::ShaderC(e)
    }
}

/// Info necessary to compile a shader from source code stored in the filesystem.
#[derive(Clone, Copy, Debug)]
pub struct FileShaderInfo<P, E> {
    path: P,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<P, E> FileShaderInfo<P, E> {
    /// Create shader info that will be compiled from the contents of `path`.
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
    pub fn precompile(&self) -> Result<SpirvShader, <Self as Shader>::Error>
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
    type Error = ShaderCError;

    fn spirv(&self) -> Result<std::borrow::Cow<'static, [u32]>, ShaderCError> {
        let code = std::fs::read_to_string(&self.path)?;

        let artifact = shaderc::Compiler::new()
            .ok_or(ShaderCError::Init)?
            .compile_into_spirv(
                &code,
                self.kind,
                self.path
                    .as_ref()
                    .to_str()
                    .ok_or_else(|| ShaderCError::NonUtf8Path(self.path.as_ref().to_owned()))?,
                self.entry.as_ref(),
                Some({
                    let mut ops = shaderc::CompileOptions::new().ok_or(ShaderCError::Init)?;
                    ops.set_target_env(shaderc::TargetEnv::Vulkan, vk_make_version!(1, 0, 0));
                    ops.set_source_language(self.lang);
                    ops.set_generate_debug_info();
                    ops.set_optimization_level(shaderc::OptimizationLevel::Performance);
                    ops
                })
                .as_ref(),
            )?;

        Ok(std::borrow::Cow::Owned(artifact.as_binary().into()))
    }

    fn entry(&self) -> &str {
        self.entry.as_ref()
    }

    fn stage(&self) -> rendy_core::hal::pso::ShaderStageFlags {
        stage_from_kind(&self.kind)
    }
}

/// Info necessary to compile a shader from provided source code.
#[derive(Clone, Copy, Debug)]
pub struct SourceCodeShaderInfo<P, E, S> {
    source: S,
    path: P,
    kind: ShaderKind,
    lang: SourceLanguage,
    entry: E,
}

impl<P, E, S> SourceCodeShaderInfo<P, E, S> {
    /// Create shader info that will be compiled from the provided `source`. Note that `path` is
    /// just a name used for diagnostics, and isn't required to be an actual file.
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
    pub fn precompile(&self) -> Result<SpirvShader, <Self as Shader>::Error>
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
    type Error = ShaderCError;

    fn spirv(&self) -> Result<std::borrow::Cow<'static, [u32]>, ShaderCError> {
        let artifact = shaderc::Compiler::new()
            .ok_or(ShaderCError::Init)?
            .compile_into_spirv(
                self.source.as_ref(),
                self.kind,
                self.path
                    .as_ref()
                    .to_str()
                    .ok_or_else(|| ShaderCError::NonUtf8Path(self.path.as_ref().to_owned()))?,
                self.entry.as_ref(),
                Some({
                    let mut ops = shaderc::CompileOptions::new().ok_or(ShaderCError::Init)?;
                    ops.set_target_env(shaderc::TargetEnv::Vulkan, vk_make_version!(1, 0, 0));
                    ops.set_source_language(self.lang);
                    ops.set_generate_debug_info();
                    ops.set_optimization_level(shaderc::OptimizationLevel::Performance);
                    ops
                })
                .as_ref(),
            )?;

        Ok(std::borrow::Cow::Owned(artifact.as_binary().into()))
    }

    fn entry(&self) -> &str {
        self.entry.as_ref()
    }

    fn stage(&self) -> rendy_core::hal::pso::ShaderStageFlags {
        stage_from_kind(&self.kind)
    }
}

/// Shader info with static data.
pub type SourceShaderInfo = SourceCodeShaderInfo<&'static str, &'static str, &'static str>;

/// DEPRECATED. USE `PathBufShaderInfo` INSTEAD!
#[deprecated(
    since = "0.2.1",
    note = "StaticShaderInfo will be removed in favor of PathBufShaderInfo soon. Please move to that implementation."
)]
pub type StaticShaderInfo = FileShaderInfo<&'static str, &'static str>;

/// Shader info with a PathBuf for the path and static string for entry
pub type PathBufShaderInfo = FileShaderInfo<std::path::PathBuf, &'static str>;

fn stage_from_kind(kind: &ShaderKind) -> rendy_core::hal::pso::ShaderStageFlags {
    use rendy_core::hal::pso::ShaderStageFlags;
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
