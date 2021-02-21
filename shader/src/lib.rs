//! Shader compilation.

#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

#[cfg(feature = "shader-compiler")]
mod shaderc;

#[cfg(feature = "spirv-reflection")]
#[allow(dead_code)]
mod reflect;

#[cfg(feature = "shader-compiler")]
pub use self::shaderc::*;

#[cfg(feature = "spirv-reflection")]
pub use self::reflect::{ReflectError, ReflectTypeError, RetrievalKind, SpirvReflection};

use rendy_core::hal::{pso::ShaderStageFlags, Backend};
use std::collections::HashMap;

use gfx_auxil::read_spirv;

/// Error type returned by this module.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShaderError {}

impl std::error::Error for ShaderError {}
impl std::fmt::Display for ShaderError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

/// Interface to create shader modules from shaders.
/// Implemented for static shaders via [`compile_to_spirv!`] macro.
///
pub trait Shader {
    /// The error type returned by the spirv function of this shader.
    type Error: std::fmt::Debug;

    /// Get spirv bytecode.
    fn spirv(&self) -> Result<std::borrow::Cow<'_, [u32]>, <Self as Shader>::Error>;

    /// Get the entry point of the shader.
    fn entry(&self) -> &str;

    /// Get the rendy_core::hal representation of this shaders kind/stage.
    fn stage(&self) -> ShaderStageFlags;

    /// Create shader module.
    ///
    /// Spir-V bytecode must adhere valid usage on this Vulkan spec page:
    /// https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkShaderModuleCreateInfo.html
    unsafe fn module<B>(
        &self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<B::ShaderModule, rendy_core::hal::device::ShaderError>
    where
        B: Backend,
    {
        rendy_core::hal::device::Device::create_shader_module(
            factory.device().raw(),
            &self.spirv().map_err(|e| {
                rendy_core::hal::device::ShaderError::CompilationFailed(format!("{:?}", e))
            })?,
        )
    }
}

/// Spir-V shader.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpirvShader {
    #[cfg_attr(feature = "serde", serde(with = "serde_spirv"))]
    spirv: Vec<u32>,
    stage: ShaderStageFlags,
    entry: String,
}

#[cfg(feature = "serde")]
mod serde_spirv {
    pub fn serialize<S>(data: &Vec<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(rendy_core::cast_slice(&data))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Via the serde::Deserialize impl for &[u8].
        let bytes: &[u8] = serde::Deserialize::deserialize(deserializer)?;
        read_spirv(std::io::Cursor::new(bytes))
            .map_err(serde::de::Error::custom)
    }
}

impl SpirvShader {
    /// Create Spir-V shader from bytes.
    pub fn new(spirv: Vec<u32>, stage: ShaderStageFlags, entrypoint: &str) -> Self {
        assert!(!spirv.is_empty());
        Self {
            spirv,
            stage,
            entry: entrypoint.to_string(),
        }
    }

    /// Create Spir-V shader from bytecode stored as bytes.
    /// Errors when passed byte array length is not a multiple of 4.
    pub fn from_bytes(
        spirv: &[u8],
        stage: ShaderStageFlags,
        entrypoint: &str,
    ) -> std::io::Result<Self> {
        Ok(Self::new(
            read_spirv(std::io::Cursor::new(spirv))?,
            stage,
            entrypoint,
        ))
    }
}

impl Shader for SpirvShader {
    type Error = ShaderError;

    fn spirv(&self) -> Result<std::borrow::Cow<'_, [u32]>, ShaderError> {
        Ok(std::borrow::Cow::Borrowed(&self.spirv))
    }

    fn entry(&self) -> &str {
        &self.entry
    }

    fn stage(&self) -> ShaderStageFlags {
        self.stage
    }
}

/// A `ShaderSet` object represents a merged collection of `ShaderStorage` structures, which reflects merged information for all shaders in the set.
#[derive(Debug)]
pub struct ShaderSet<B: Backend> {
    shaders: HashMap<ShaderStageFlags, ShaderStorage<B>>,
}

impl<B> Default for ShaderSet<B>
where
    B: Backend,
{
    fn default() -> Self {
        ShaderSet {
            shaders: HashMap::default(),
        }
    }
}

impl<B: Backend> ShaderSet<B> {
    /// This function compiles and loads all shaders into B::ShaderModule objects which must be dropped later with `dispose`
    pub fn load(
        &mut self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<&mut Self, rendy_core::hal::device::ShaderError> {
        for (_, v) in self.shaders.iter_mut() {
            unsafe { v.compile(factory)? }
        }

        Ok(self)
    }

    pub fn raw_vertex<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(&ShaderStageFlags::VERTEX) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    pub fn raw_fragment<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(&ShaderStageFlags::FRAGMENT) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    pub fn raw_domain<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(&ShaderStageFlags::DOMAIN) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    pub fn raw_hull<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(&ShaderStageFlags::HULL) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    pub fn raw_geometry<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(&ShaderStageFlags::GEOMETRY) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    pub fn raw_mesh<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(&ShaderStageFlags::MESH) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    pub fn raw_task<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(&ShaderStageFlags::TASK) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    /// Must be called to perform a drop of the Backend ShaderModule object otherwise the shader will never be destroyed in memory.
    pub fn dispose(&mut self, factory: &rendy_factory::Factory<B>) {
        for (_, shader) in self.shaders.iter_mut() {
            shader.dispose(factory);
        }
    }
}

/// A set of Specialization constants for a certain shader set.
#[derive(Debug, Default, Clone)]
#[allow(missing_copy_implementations)]
pub struct SpecConstantSet {
    /// Vertex specialization
    pub vertex: Option<rendy_core::hal::pso::Specialization<'static>>,
    /// Fragment specialization
    pub fragment: Option<rendy_core::hal::pso::Specialization<'static>>,
    /// Geometry specialization
    pub geometry: Option<rendy_core::hal::pso::Specialization<'static>>,
    /// Hull specialization
    pub hull: Option<rendy_core::hal::pso::Specialization<'static>>,
    /// Domain specialization
    pub domain: Option<rendy_core::hal::pso::Specialization<'static>>,
    /// Compute specialization
    pub compute: Option<rendy_core::hal::pso::Specialization<'static>>,
}

/// Builder class which is used to begin the reflection and shader set construction process for a shader set. Provides all the functionality needed to
/// build a shader set with provided shaders and then reflect appropriate gfx-hal and generic shader information.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShaderSetBuilder {
    vertex: Option<(Vec<u32>, String)>,
    fragment: Option<(Vec<u32>, String)>,
    geometry: Option<(Vec<u32>, String)>,
    hull: Option<(Vec<u32>, String)>,
    domain: Option<(Vec<u32>, String)>,
    compute: Option<(Vec<u32>, String)>,
}

impl ShaderSetBuilder {
    /// Builds the Backend-specific shader modules using the provided SPIRV code provided to the builder.
    /// This function is called during the creation of a render pass.
    ///
    /// # Parameters
    ///
    /// `factory`   - factory to create shader modules.
    ///
    pub fn build<B: Backend>(
        &self,
        factory: &rendy_factory::Factory<B>,
        spec_constants: SpecConstantSet,
    ) -> Result<ShaderSet<B>, rendy_core::hal::device::ShaderError> {
        let mut set = ShaderSet::<B>::default();

        if self.vertex.is_none() && self.compute.is_none() {
            let msg = "A vertex or compute shader must be provided".to_string();
            return Err(rendy_core::hal::device::ShaderError::InterfaceMismatch(msg));
        }
        type ShaderTy = (
            Vec<u32>,
            String,
            Option<rendy_core::hal::pso::Specialization<'static>>,
        );

        let create_storage =
            move |stage,
                  shader: ShaderTy,
                  factory|
                  -> Result<ShaderStorage<B>, rendy_core::hal::device::ShaderError> {
                let mut storage = ShaderStorage {
                    stage,
                    spirv: shader.0,
                    module: None,
                    entrypoint: shader.1.clone(),
                    specialization: shader.2,
                };
                unsafe {
                    storage.compile(factory)?;
                }
                Ok(storage)
            };

        if let Some(shader) = self.vertex.clone() {
            set.shaders.insert(
                ShaderStageFlags::VERTEX,
                create_storage(
                    ShaderStageFlags::VERTEX,
                    (shader.0, shader.1, spec_constants.vertex),
                    factory,
                )?,
            );
        }

        if let Some(shader) = self.fragment.clone() {
            set.shaders.insert(
                ShaderStageFlags::FRAGMENT,
                create_storage(
                    ShaderStageFlags::FRAGMENT,
                    (shader.0, shader.1, spec_constants.fragment),
                    factory,
                )?,
            );
        }

        if let Some(shader) = self.compute.clone() {
            set.shaders.insert(
                ShaderStageFlags::COMPUTE,
                create_storage(
                    ShaderStageFlags::COMPUTE,
                    (shader.0, shader.1, spec_constants.compute),
                    factory,
                )?,
            );
        }

        if let Some(shader) = self.domain.clone() {
            set.shaders.insert(
                ShaderStageFlags::DOMAIN,
                create_storage(
                    ShaderStageFlags::DOMAIN,
                    (shader.0, shader.1, spec_constants.domain),
                    factory,
                )?,
            );
        }

        if let Some(shader) = self.hull.clone() {
            set.shaders.insert(
                ShaderStageFlags::HULL,
                create_storage(
                    ShaderStageFlags::HULL,
                    (shader.0, shader.1, spec_constants.hull),
                    factory,
                )?,
            );
        }

        if let Some(shader) = self.geometry.clone() {
            set.shaders.insert(
                ShaderStageFlags::GEOMETRY,
                create_storage(
                    ShaderStageFlags::GEOMETRY,
                    (shader.0, shader.1, spec_constants.geometry),
                    factory,
                )?,
            );
        }

        Ok(set)
    }

    /// Add a vertex shader to this shader set
    #[inline(always)]
    pub fn with_vertex<S: Shader>(mut self, shader: &S) -> Result<Self, S::Error> {
        let data = shader.spirv()?;
        self.vertex = Some((data.to_vec(), shader.entry().to_string()));
        Ok(self)
    }

    /// Add a fragment shader to this shader set
    #[inline(always)]
    pub fn with_fragment<S: Shader>(mut self, shader: &S) -> Result<Self, S::Error> {
        let data = shader.spirv()?;
        self.fragment = Some((data.to_vec(), shader.entry().to_string()));
        Ok(self)
    }

    /// Add a geometry shader to this shader set
    #[inline(always)]
    pub fn with_geometry<S: Shader>(mut self, shader: &S) -> Result<Self, S::Error> {
        let data = shader.spirv()?;
        self.geometry = Some((data.to_vec(), shader.entry().to_string()));
        Ok(self)
    }

    /// Add a hull shader to this shader set
    #[inline(always)]
    pub fn with_hull<S: Shader>(mut self, shader: &S) -> Result<Self, S::Error> {
        let data = shader.spirv()?;
        self.hull = Some((data.to_vec(), shader.entry().to_string()));
        Ok(self)
    }

    /// Add a domain shader to this shader set
    #[inline(always)]
    pub fn with_domain<S: Shader>(mut self, shader: &S) -> Result<Self, S::Error> {
        let data = shader.spirv()?;
        self.domain = Some((data.to_vec(), shader.entry().to_string()));
        Ok(self)
    }

    /// Add a compute shader to this shader set.
    /// Note a compute or vertex shader must always exist in a shader set.
    #[inline(always)]
    pub fn with_compute<S: Shader>(mut self, shader: &S) -> Result<Self, S::Error> {
        let data = shader.spirv()?;
        self.compute = Some((data.to_vec(), shader.entry().to_string()));
        Ok(self)
    }

    #[cfg(feature = "spirv-reflection")]
    /// This function processes all shaders provided to the builder and computes and stores full reflection information on the shader.
    /// This includes names, attributes, descriptor sets and push constants used by the shaders, as well as compiling local caches for performance.
    pub fn reflect(&self) -> Result<SpirvReflection, ReflectError> {
        if self.vertex.is_none() && self.compute.is_none() {
            return Err(ReflectError::NoVertComputeProvided);
        }

        // We need to combine and merge all the reflections into a single SpirvReflection instance
        let mut reflections = Vec::new();
        if let Some(vertex) = self.vertex.as_ref() {
            reflections.push(SpirvReflection::reflect(&vertex.0, None)?);
        }
        if let Some(fragment) = self.fragment.as_ref() {
            reflections.push(SpirvReflection::reflect(&fragment.0, None)?);
        }
        if let Some(hull) = self.hull.as_ref() {
            reflections.push(SpirvReflection::reflect(&hull.0, None)?);
        }
        if let Some(domain) = self.domain.as_ref() {
            reflections.push(SpirvReflection::reflect(&domain.0, None)?);
        }
        if let Some(compute) = self.compute.as_ref() {
            reflections.push(SpirvReflection::reflect(&compute.0, None)?);
        }
        if let Some(geometry) = self.geometry.as_ref() {
            reflections.push(SpirvReflection::reflect(&geometry.0, None)?);
        }

        reflect::merge(&reflections)?.compile_cache()
    }
}

/// Contains reflection and runtime nformation for a given compiled Shader Module.
#[derive(Debug)]
pub struct ShaderStorage<B: Backend> {
    stage: ShaderStageFlags,
    spirv: Vec<u32>,
    module: Option<B::ShaderModule>,
    entrypoint: String,
    specialization: Option<rendy_core::hal::pso::Specialization<'static>>,
}
impl<B: Backend> ShaderStorage<B> {
    /// Builds the `EntryPoint` structure used by rendy_core::hal to determine how to utilize this shader
    pub fn get_entry_point<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        Ok(Some(rendy_core::hal::pso::EntryPoint {
            entry: &self.entrypoint,
            module: self.module.as_ref().unwrap(),
            specialization: self.specialization.clone().unwrap_or_default(),
        }))
    }

    /// Compile the SPIRV code with the backend and store the reference to the module inside this structure.
    pub unsafe fn compile(
        &mut self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<(), rendy_core::hal::device::ShaderError> {
        self.module = Some(rendy_core::hal::device::Device::create_shader_module(
            factory.device().raw(),
            &self.spirv,
        )?);

        Ok(())
    }

    fn dispose(&mut self, factory: &rendy_factory::Factory<B>) {
        use rendy_core::hal::device::Device;

        if let Some(module) = self.module.take() {
            unsafe { factory.destroy_shader_module(module) };
        }
        self.module = None;
    }
}

impl<B: Backend> Drop for ShaderStorage<B> {
    fn drop(&mut self) {
        if self.module.is_some() {
            panic!("This shader storage class needs to be manually dropped with dispose() first");
        }
    }
}
