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
pub use self::reflect::{SpirvReflection, SpirvReflectionGenerator};

#[cfg(not(feature = "spirv-reflection"))]
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
struct SpirvReflection {
    entrypoints: Vec<(ShaderStageFlags, String)>,
    entrypoint: Option<String>,
}

use gfx_hal::{pso::ShaderStageFlags, Backend};
use std::collections::HashMap;

/// Interface to create shader modules from shaders.
/// Implemented for static shaders via [`compile_to_spirv!`] macro.
///
pub trait Shader {
    /// Get spirv bytecode.
    fn spirv(&self) -> Result<std::borrow::Cow<'_, [u8]>, failure::Error>;

    /// Get the entry point of the shader.
    fn entry(&self) -> &str;

    /// Get the gfx_hal representation of this shaders kind/stage.
    fn stage(&self) -> ShaderStageFlags;

    /// Create shader module.
    ///
    /// Spir-V bytecode must adhere valid usage on this Vulkan spec page:
    /// https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkShaderModuleCreateInfo.html
    unsafe fn module<B>(
        &self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<B::ShaderModule, failure::Error>
    where
        B: Backend,
    {
        gfx_hal::Device::create_shader_module(factory.device().raw(), &self.spirv()?)
            .map_err(Into::into)
    }
}

/// Spir-V shader.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpirvShader {
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    spirv: Vec<u8>,
    stage: ShaderStageFlags,
    entry: String,
}

impl SpirvShader {
    /// Create Spir-V shader from bytes.
    pub fn new(spirv: Vec<u8>, stage: ShaderStageFlags, entrypoint: &str) -> Self {
        assert!(!spirv.is_empty());
        assert_eq!(spirv.len() % 4, 0);
        Self {
            spirv,
            stage,
            entry: entrypoint.to_string(),
        }
    }
}

impl Shader for SpirvShader {
    fn spirv(&self) -> Result<std::borrow::Cow<'_, [u8]>, failure::Error> {
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
#[derive(derivative::Derivative, Debug)]
#[derivative(Default(bound = ""))]
pub struct ShaderSet<B: Backend> {
    shaders: HashMap<ShaderStageFlags, ShaderStorage<B>>,
    set_reflection: SpirvReflection,
}

impl<B: Backend> ShaderSet<B> {
    /// This function compiles and loads all shaders into B::ShaderModule objects which must be dropped later with `dispose`
    pub fn load(
        &mut self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<&mut Self, failure::Error> {
        for (_, v) in self.shaders.iter_mut() {
            unsafe { v.compile(factory)? }
        }

        Ok(self)
    }

    /// Returns the `GraphicsShaderSet` structure to provide all the runtime information needed to use the shaders in this set in gfx_hal.
    pub fn raw<'a>(&'a self) -> Result<(gfx_hal::pso::GraphicsShaderSet<'a, B>), failure::Error> {
        Ok(gfx_hal::pso::GraphicsShaderSet {
            vertex: self
                .shaders
                .get(&ShaderStageFlags::VERTEX)
                .unwrap()
                .get_entry_point()?
                .unwrap(),
            fragment: match self.shaders.get(&ShaderStageFlags::FRAGMENT) {
                Some(fragment) => fragment.get_entry_point()?,
                None => None,
            },
            domain: match self.shaders.get(&ShaderStageFlags::DOMAIN) {
                Some(domain) => domain.get_entry_point()?,
                None => None,
            },
            hull: match self.shaders.get(&ShaderStageFlags::HULL) {
                Some(hull) => hull.get_entry_point()?,
                None => None,
            },
            geometry: match self.shaders.get(&ShaderStageFlags::GEOMETRY) {
                Some(geometry) => geometry.get_entry_point()?,
                None => None,
            },
        })
    }

    /// Must be called to perform a drop of the Backend ShaderModule object otherwise the shader will never be destroyed in memory.
    pub fn dispose(&mut self, factory: &rendy_factory::Factory<B>) {
        for (_, shader) in self.shaders.iter_mut() {
            shader.dispose(factory);
        }
    }
}

/// Builder class which is used to begin the reflection and shader set construction process for a shader set. Provides all the functionality needed to
/// build a shader set with provided shaders and then reflect appropriate gfx-hal and generic shader information.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShaderSetBuilder {
    vertex: Option<(Vec<u8>, SpirvReflection)>,
    fragment: Option<(Vec<u8>, SpirvReflection)>,
    geometry: Option<(Vec<u8>, SpirvReflection)>,
    hull: Option<(Vec<u8>, SpirvReflection)>,
    domain: Option<(Vec<u8>, SpirvReflection)>,
    compute: Option<(Vec<u8>, SpirvReflection)>,
    set_reflection: Option<SpirvReflection>,
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
    ) -> Result<ShaderSet<B>, failure::Error> {
        let mut set = ShaderSet::<B>::default();

        if self.vertex.is_none() && self.compute.is_none() {
            failure::bail!("A vertex or compute shader must be provided");
        }

        let create_storage = move |stage,
                                   shader: (Vec<u8>, SpirvReflection),
                                   factory|
              -> Result<ShaderStorage<B>, failure::Error> {
            let entry = shader.1.entrypoint.clone();
            let mut storage = ShaderStorage {
                stage: stage,
                spirv: shader.0,
                reflection: shader.1,
                module: None,
                entrypoint: entry.unwrap(),
            };
            unsafe {
                storage.compile(factory)?;
            }
            Ok(storage)
        };

        if let Some(shader) = self.vertex.clone() {
            set.shaders.insert(
                ShaderStageFlags::VERTEX,
                create_storage(ShaderStageFlags::VERTEX, shader, factory)?,
            );
        }

        if let Some(shader) = self.fragment.clone() {
            set.shaders.insert(
                ShaderStageFlags::FRAGMENT,
                create_storage(ShaderStageFlags::FRAGMENT, shader, factory)?,
            );
        }

        if let Some(shader) = self.compute.clone() {
            set.shaders.insert(
                ShaderStageFlags::COMPUTE,
                create_storage(ShaderStageFlags::COMPUTE, shader, factory)?,
            );
        }

        if let Some(shader) = self.domain.clone() {
            set.shaders.insert(
                ShaderStageFlags::DOMAIN,
                create_storage(ShaderStageFlags::DOMAIN, shader, factory)?,
            );
        }

        if let Some(shader) = self.hull.clone() {
            set.shaders.insert(
                ShaderStageFlags::HULL,
                create_storage(ShaderStageFlags::HULL, shader, factory)?,
            );
        }

        if let Some(shader) = self.geometry.clone() {
            set.shaders.insert(
                ShaderStageFlags::GEOMETRY,
                create_storage(ShaderStageFlags::GEOMETRY, shader, factory)?,
            );
        }

        Ok(set)
    }

    /// Add a vertex shader to this shader set
    #[inline(always)]
    pub fn with_vertex<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.vertex = Some((
            data.to_vec(),
            self.reflect_shader(shader.stage(), shader.entry(), data)?,
        ));
        Ok(self)
    }

    /// Add a fragment shader to this shader set
    #[inline(always)]
    pub fn with_fragment<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.fragment = Some((
            data.to_vec(),
            self.reflect_shader(shader.stage(), shader.entry(), data)?,
        ));
        Ok(self)
    }

    /// Add a geometry shader to this shader set
    #[inline(always)]
    pub fn with_geometry<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.geometry = Some((
            data.to_vec(),
            self.reflect_shader(shader.stage(), shader.entry(), data)?,
        ));
        Ok(self)
    }

    /// Add a hull shader to this shader set
    #[inline(always)]
    pub fn with_hull<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.hull = Some((
            data.to_vec(),
            self.reflect_shader(shader.stage(), shader.entry(), data)?,
        ));
        Ok(self)
    }

    /// Add a domain shader to this shader set
    #[inline(always)]
    pub fn with_domain<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.domain = Some((
            data.to_vec(),
            self.reflect_shader(shader.stage(), shader.entry(), data)?,
        ));
        Ok(self)
    }

    /// Add a compute shader to this shader set.
    /// Note a compute or vertex shader must always exist in a shader set.
    #[inline(always)]
    pub fn with_compute<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.compute = Some((
            data.to_vec(),
            self.reflect_shader(shader.stage(), shader.entry(), data)?,
        ));
        Ok(self)
    }

    #[cfg(feature = "spirv-reflection")]
    #[inline(always)]
    fn reflect_shader(
        &mut self,
        _: ShaderStageFlags,
        entrypoint: &str,
        data: std::borrow::Cow<'_, [u8]>,
    ) -> Result<SpirvReflection, failure::Error> {
        Ok(SpirvReflection::reflect(&data, Some(entrypoint))?)
    }

    #[cfg(not(feature = "spirv-reflection"))]
    #[inline(always)]
    fn reflect_shader(
        &mut self,
        stage: ShaderStageFlags,
        entrypoint: &str,
        _: std::borrow::Cow<'_, [u8]>,
    ) -> Result<SpirvReflection, failure::Error> {
        Ok(SpirvReflection {
            entrypoints: vec![(stage, entrypoint.to_string())],
            entrypoint: Some(entrypoint.to_string()),
        })
    }

    #[cfg(feature = "spirv-reflection")]
    /// This function processes all shaders provided to the builder and computes and stores full reflection information on the shader.
    /// This includes names, attributes, descriptor sets and push constants used by the shaders, as well as compiling local caches for performance.
    pub fn reflect(mut self) -> Result<Self, failure::Error> {
        if self.vertex.is_none() && self.compute.is_none() {
            failure::bail!("A vertex or compute shader must be provided");
        }

        // We need to combine and merge all the reflections into a single SpirvReflection instance
        let mut reflections = Vec::new();
        if let Some(vertex) = self.vertex.as_mut() {
            vertex.1 = SpirvReflection::reflect(&vertex.0, None)?;
            reflections.push(&vertex.1);
        }
        if let Some(fragment) = self.fragment.as_mut() {
            fragment.1 = SpirvReflection::reflect(&fragment.0, None)?;
            reflections.push(&fragment.1);
        }
        if let Some(hull) = self.hull.as_mut() {
            hull.1 = SpirvReflection::reflect(&hull.0, None)?;
            reflections.push(&hull.1);
        }
        if let Some(domain) = self.domain.as_mut() {
            domain.1 = SpirvReflection::reflect(&domain.0, None)?;
            reflections.push(&domain.1);
        }
        if let Some(compute) = self.compute.as_mut() {
            compute.1 = SpirvReflection::reflect(&compute.0, None)?;
            reflections.push(&compute.1);
        }
        if let Some(geometry) = self.geometry.as_mut() {
            geometry.1 = SpirvReflection::reflect(&geometry.0, None)?;
            reflections.push(&geometry.1);
        }

        self.set_reflection = Some(reflect::merge(&reflections)?);
        self.set_reflection.as_mut().unwrap().compile_cache()?;

        Ok(self)
    }
}

/// Contains reflection and runtime nformation for a given compiled Shader Module.
#[derive(Debug)]
pub struct ShaderStorage<B: Backend> {
    stage: ShaderStageFlags,
    spirv: Vec<u8>,
    reflection: SpirvReflection,
    module: Option<B::ShaderModule>,
    entrypoint: String,
}
impl<B: Backend> ShaderStorage<B> {
    /// Builds the `EntryPoint` structure used by gfx_hal to determine how to utilize this shader
    pub fn get_entry_point<'a>(
        &'a self,
    ) -> Result<Option<gfx_hal::pso::EntryPoint<'a, B>>, failure::Error> {
        Ok(Some(gfx_hal::pso::EntryPoint {
            entry: &self
                .reflection
                .entrypoints
                .iter()
                .find(|e| e.0 == self.stage && e.1 == self.entrypoint)
                .ok_or(failure::format_err!(
                    "Shader {:?} missing entry point {}",
                    self.stage,
                    self.entrypoint
                ))?
                .1,
            module: self.module.as_ref().unwrap(),
            specialization: gfx_hal::pso::Specialization::default(),
        }))
    }

    /// Compile the SPIRV code with the backend and store the reference to the module inside this structure.
    pub unsafe fn compile(
        &mut self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<(), failure::Error> {
        self.module = Some(gfx_hal::Device::create_shader_module(
            factory.device().raw(),
            &self.spirv,
        )?);

        Ok(())
    }

    fn dispose(&mut self, factory: &rendy_factory::Factory<B>) {
        use gfx_hal::device::Device;

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
