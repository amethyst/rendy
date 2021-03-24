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

use std::ops::Range;
use std::hash::Hash;

#[cfg(feature = "shader-compiler")]
mod shaderc;

#[cfg(feature = "spirv-reflection")]
#[allow(dead_code)]
mod reflect;

mod stage_map;
pub use stage_map::{ShaderStage, StageMap};

mod id;
pub use id::ShaderId;

#[cfg(feature = "shader-compiler")]
pub use self::shaderc::*;

#[cfg(feature = "spirv-reflection")]
pub use self::reflect::{ReflectError, ReflectTypeError, RetrievalKind, SpirvReflection};

use rendy_core::hal::{pso::ShaderStageFlags, Backend};
use rendy_core::hal::device::{Device as _, OutOfMemory};

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
    id: ShaderId,
    shaders: StageMap<ShaderStorage<B>>,
    pipeline_layout: PipelineLayout<B>,
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

    /// Returns the unique ID for this shader set
    pub fn shader_id(&self) -> ShaderId {
        self.id
    }

    /// Gets the specified raw shader stage entry point
    #[inline]
    pub fn get_raw<'a>(
        &'a self,
        stage: ShaderStage,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        if let Some(e) = self.shaders.get(stage) {
            Ok(Some(e.get_entry_point()?.unwrap()))
        } else {
            Ok(None)
        }
    }

    /// Gets the raw vertex shader entry point, if there is one
    pub fn raw_vertex<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Vertex)
    }

    /// Gets the raw fragment shader entry point, if there is one
    pub fn raw_fragment<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Fragment)
    }

    /// Gets the raw domain shader entry point, if there is one
    pub fn raw_domain<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Domain)
    }

    /// Gets the raw hull shader entry point, if there is one
    pub fn raw_hull<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Hull)
    }

    /// Gets the raw geometry shader entry point, if there is one
    pub fn raw_geometry<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Geometry)
    }

    /// Gets the raw geometry shader entry point, if there is one
    pub fn raw_compute<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Compute)
    }

    /// Gets the raw mesh shader entry point, if there is one
    pub fn raw_mesh<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Mesh)
    }

    /// Gets the raw task shader entry point, if there is one
    pub fn raw_task<'a>(
        &'a self,
    ) -> Result<Option<rendy_core::hal::pso::EntryPoint<'a, B>>, ShaderError> {
        self.get_raw(ShaderStage::Task)
    }

    pub fn pipeline_layout(&self) -> &PipelineLayout<B> {
        &self.pipeline_layout
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
    /// Map of stagewise specialisation data for a shader set
    pub stages: StageMap<rendy_core::hal::pso::Specialization<'static>>,
}

// TODO derive for pso::Specialization upstream
impl Hash for SpecConstantSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (stage, data) in self.stages.iter() {
            stage.hash(state);
            data.constants.hash(state);
            data.data.hash(state);
        }
    }
}
impl PartialEq for SpecConstantSet {
    fn eq(&self, rhs: &Self) -> bool {
        for ((_l_stage, l_data), (_r_stage, r_data)) in self.stages.iter_all().zip(rhs.stages.iter_all()) {
            if l_data.is_none() != r_data.is_none() {
                return false;
            }
            if let (Some(l_data), Some(r_data)) = (l_data, r_data) {
                if l_data.constants != r_data.constants || l_data.data != r_data.data {
                    return false;
                }
            }
        }
        true
    }
}
impl Eq for SpecConstantSet {}

#[derive(Debug, Clone)]
pub struct PipelineLayoutDescr {
    pub descriptor_sets: Vec<Vec<rendy_core::hal::pso::DescriptorSetLayoutBinding>>,
    pub push_constants: Vec<(ShaderStageFlags, Range<u32>)>,
}
impl PipelineLayoutDescr {
    pub fn from_reflect(reflect: &SpirvReflection) -> Self {
        Self {
            descriptor_sets: reflect.descriptor_sets.clone(),
            push_constants: reflect.push_constants.clone()
        }
    }

    pub fn create<B: Backend>(&self, device: &rendy_core::Device<B>) -> Result<PipelineLayout<B>, OutOfMemory> {
        let set_layouts = self
            .descriptor_sets
            .iter()
            .map(|b| {
                unsafe {
                    device.raw().create_descriptor_set_layout(
                        b.iter().cloned(), std::iter::empty())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let pipeline_layout = unsafe {
            device.raw().create_pipeline_layout(
                set_layouts.iter(), self.push_constants.iter().cloned())?
        };

        Ok(PipelineLayout {
            set_layouts,
            pipeline_layout,
        })
    }
}

#[derive(Debug)]
pub struct PipelineLayout<B: Backend> {
    set_layouts: Vec<B::DescriptorSetLayout>,
    pipeline_layout: B::PipelineLayout,
}
impl<B: Backend> PipelineLayout<B> {
    pub fn raw(&self) -> &B::PipelineLayout {
        &self.pipeline_layout
    }
}

/// Struct which contains the source for a set of shaders for a pipeline.
///
/// Can be built in a particular vulkan instance, reflected upon or hashed and
/// stored.
#[derive(Clone, Debug, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShaderSourceSet {
    stages: StageMap<(Vec<u32>, String)>,
}

impl ShaderSourceSet {
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
        pipeline_layout: PipelineLayoutDescr,
        spec_constants: SpecConstantSet,
    ) -> Result<ShaderSet<B>, rendy_core::hal::device::ShaderError> {
        if self.stages.get(ShaderStage::Vertex).is_none() && self.stages.get(ShaderStage::Compute).is_none() {
            let msg = "A vertex or compute shader must be provided".to_string();
            return Err(rendy_core::hal::device::ShaderError::CompilationFailed(msg));
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

        let mut stages = StageMap::new();
        for (stage, shader) in self.stages.iter() {
            let shader = shader.clone();

            let storage = create_storage(
                stage.into(),
                (shader.0, shader.1, spec_constants.stages.get(stage).cloned()),
                factory,
            )?;

            stages.insert(stage, storage);
        }

        let pipeline_layout = pipeline_layout.create(factory.device())?;

        Ok(ShaderSet {
            id: ShaderId::generate(),
            shaders: stages,
            pipeline_layout,
        })
    }

    /// Add the specified stage to this shader set
    #[inline(always)]
    pub fn with_stage<S: Shader>(mut self, stage: ShaderStage, shader: &S) -> Result<Self, S::Error> {
        let data = shader.spirv()?;
        self.stages.insert(stage, (data.to_vec(), shader.entry().to_string()));
        Ok(self)
    }

    /// Add a vertex shader to this shader set
    #[inline(always)]
    pub fn with_vertex<S: Shader>(self, shader: &S) -> Result<Self, S::Error> {
        self.with_stage(ShaderStage::Vertex, shader)
    }

    /// Add a fragment shader to this shader set
    #[inline(always)]
    pub fn with_fragment<S: Shader>(self, shader: &S) -> Result<Self, S::Error> {
        self.with_stage(ShaderStage::Fragment, shader)
    }

    /// Add a geometry shader to this shader set
    #[inline(always)]
    pub fn with_geometry<S: Shader>(self, shader: &S) -> Result<Self, S::Error> {
        self.with_stage(ShaderStage::Geometry, shader)
    }

    /// Add a hull shader to this shader set
    #[inline(always)]
    pub fn with_hull<S: Shader>(self, shader: &S) -> Result<Self, S::Error> {
        self.with_stage(ShaderStage::Hull, shader)
    }

    /// Add a domain shader to this shader set
    #[inline(always)]
    pub fn with_domain<S: Shader>(self, shader: &S) -> Result<Self, S::Error> {
        self.with_stage(ShaderStage::Domain, shader)
    }

    /// Add a compute shader to this shader set.
    /// Note a compute or vertex shader must always exist in a shader set.
    #[inline(always)]
    pub fn with_compute<S: Shader>(self, shader: &S) -> Result<Self, S::Error> {
        self.with_stage(ShaderStage::Compute, shader)
    }

    #[cfg(feature = "spirv-reflection")]
    /// This function processes all shaders provided to the builder and computes and stores full reflection information on the shader.
    /// This includes names, attributes, descriptor sets and push constants used by the shaders, as well as compiling local caches for performance.
    pub fn reflect(&self) -> Result<SpirvReflection, ReflectError> {
        if self.stages.get(ShaderStage::Vertex).is_none() && self.stages.get(ShaderStage::Compute).is_none() {
            return Err(ReflectError::NoVertComputeProvided);
        }

        // We need to combine and merge all the reflections into a single SpirvReflection instance
        let mut reflections = Vec::new();
        if let Some(vertex) = self.stages.get(ShaderStage::Vertex) {
            reflections.push(SpirvReflection::reflect(&vertex.0, None)?);
        }
        if let Some(fragment) = self.stages.get(ShaderStage::Fragment) {
            reflections.push(SpirvReflection::reflect(&fragment.0, None)?);
        }
        if let Some(hull) = self.stages.get(ShaderStage::Hull) {
            reflections.push(SpirvReflection::reflect(&hull.0, None)?);
        }
        if let Some(domain) = self.stages.get(ShaderStage::Domain) {
            reflections.push(SpirvReflection::reflect(&domain.0, None)?);
        }
        if let Some(compute) = self.stages.get(ShaderStage::Compute) {
            reflections.push(SpirvReflection::reflect(&compute.0, None)?);
        }
        if let Some(geometry) = self.stages.get(ShaderStage::Geometry) {
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
