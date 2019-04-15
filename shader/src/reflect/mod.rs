use crate::Shader;
use gfx_hal::{pso::ShaderStageFlags, Backend};
use rendy_util::types::{vertex::VertexFormat, Layout, SetLayout};
use spirv_reflect::ShaderModule;
use std::collections::HashMap;
use std::ops::{RangeBounds, Bound};
mod types;
use types::*;

#[derive(Clone, Debug)]
pub struct SpirvCachedGfxDescription {
    pub vertices: (Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>, u32),
    pub layout: Layout,
}

pub trait SpirvReflectionGenerator {
    /// Returns attributes from range
    fn attributes_range<R: std::ops::RangeBounds<usize>>(
        &self,
        range: R,
    ) -> Result<VertexFormat, failure::Error>;

    /// Returns attributes by name
    fn attributes(
        &self,
        names: &[&str],
    ) -> Result<VertexFormat, failure::Error>;

    /// Returns from full set reflection
    fn layout(&self) -> Result<Layout, failure::Error>;

    fn stage(&self) -> ShaderStageFlags;

    fn push_constants(
        &self,
        range: Option<std::ops::Range<usize>>,
    ) -> Result<Vec<(ShaderStageFlags, std::ops::Range<u32>)>, failure::Error>;
}

#[derive(Clone, Debug)]
pub struct SpirvReflection {
    /// Vec of output variables with names.
    pub output_attributes: HashMap<String, gfx_hal::pso::AttributeDesc>,
    /// Vec of output variables with names.
    pub input_attributes: HashMap<String, gfx_hal::pso::AttributeDesc>,
    /// Hashmap of output variables with names.
    pub descriptor_sets: Vec<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>>,
    /// Stage flag of this shader
    pub stage_flag: ShaderStageFlags,
    /// Push Constants
    pub push_constants: Vec<(ShaderStageFlags, std::ops::Range<u32>)>,
    /// Entrypoint name
    pub entrypoints: Vec<(ShaderStageFlags, String)>,
    /// Cached value of gfx-hal specific data
    pub cache: Option<SpirvCachedGfxDescription>,
}
impl Default for SpirvReflection {
    fn default() -> Self {
        Self {
            output_attributes: HashMap::new(),
            input_attributes: HashMap::new(),
            descriptor_sets: Vec::new(),
            stage_flag: ShaderStageFlags::VERTEX,
            push_constants: Vec::new(),
            entrypoints: Vec::new(),
            cache: None,
        }
    }
}

impl SpirvReflection {
    pub fn new(
        stage_flag: ShaderStageFlags,
        entrypoints: Vec<(ShaderStageFlags, String)>,
        input_attributes: HashMap<String, gfx_hal::pso::AttributeDesc>,
        output_attributes: HashMap<String, gfx_hal::pso::AttributeDesc>,
        descriptor_sets: Vec<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>>,
        push_constants: Vec<(ShaderStageFlags, std::ops::Range<u32>)>,
    ) -> Result<Self, failure::Error> {
        let mut selfie = SpirvReflection {
            output_attributes,
            input_attributes,
            descriptor_sets,
            stage_flag,
            push_constants,
            entrypoints,
            cache: None,
        };

        Ok(selfie)
    }

    fn compile_cache(&mut self) -> Result<(), failure::Error> {
        log::trace!("Compiling cache for set: {:?}", self.stage_flag);

        let mut stride: u32 = 0;
        let mut vertices = self
            .input_attributes
            .iter()
            .map(|(_, e)| e)
            .collect::<Vec<_>>();
        vertices.sort_by(|a, b| a.location.cmp(&b.location));
        log::trace!("Sorted: {:?}", vertices);

        self.cache = Some(SpirvCachedGfxDescription {
            vertices: (vertices.iter().map(|e| {
                let mut element = e.element.clone();
                element.offset = stride;
                stride += element.format.surface_desc().bits as u32 / 8;
                element
            }).collect::<Vec<_>>(), stride),
            layout: Layout {
                sets: vec![SetLayout {
                    bindings: self.descriptor_sets[0].clone(),
                }],
                push_constants: self.push_constants.clone(),
            },
        });

        log::trace!("Compiled Cache: {:?}", self.cache);

        Ok(())
    }

    pub fn reflect(spirv: &[u8]) -> Result<Self, failure::Error> {
        let reflected = SpirvReflection::default();

        log::trace!("Shader reflecting into SpirvReflection");

        match ShaderModule::load_u8_data(spirv) {
            Ok(module) => {
                let stage_flag = types::convert_stage(module.get_shader_stage());

                let input_attributes = types::generate_attributes(
                    module.enumerate_input_variables(None).map_err(|e| {
                        failure::format_err!(
                            "Failed to get input attributes from spirv-reflect: {}",
                            e
                        )
                    })?,
                );

                let output_attributes = types::generate_attributes(
                    module.enumerate_input_variables(None).map_err(|e| {
                        failure::format_err!(
                            "Failed to get output attributes from spirv-reflect: {}",
                            e
                        )
                    })?,
                );

                let descriptor_sets: Result<Vec<_>, _> = module
                    .enumerate_descriptor_sets(None)
                    .map_err(|e| {
                        failure::format_err!(
                            "Failed to get descriptor sets from spirv-reflect: {}",
                            e
                        )
                    })?
                    .iter()
                    .map(ReflectInto::<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>>::reflect_into)
                    .collect();

                // This is a fixup-step required because of our implementation. Because we dont pass the module around
                // to the all the reflect_into API's, we need to fix up the shader stage here at the end. Kinda a hack
                let mut descriptor_sets_final = descriptor_sets
                    .map_err(|e| failure::format_err!("Failed to parse descriptor sets:: {}", e))?;
                descriptor_sets_final.iter_mut().for_each(|v| {
                    v.iter_mut()
                        .for_each(|mut set| set.stage_flags = stage_flag);
                });

                let push_constants: Result<Vec<_>, _> = module
                    .enumerate_push_constant_blocks(None)
                    .map_err(|e| {
                        failure::format_err!(
                            "Failed to get push constants from spirv-reflect: {}",
                            e
                        )
                    })?
                    .iter()
                    .map(|c| types::convert_push_constant(stage_flag, c))
                    .collect();

                Self::new(
                    stage_flag,
                    vec![(stage_flag, module.get_entry_point_name())],
                    input_attributes.map_err(|e| {
                        failure::format_err!("Error parsing input attributes: {}", e)
                    })?,
                    output_attributes.map_err(|e| {
                        failure::format_err!("Error parsing output attributes: {}", e)
                    })?,
                    descriptor_sets_final,
                    push_constants?,
                )
            }
            Err(e) => failure::bail!("Failed to reflect data: {}", e),
        }
    }
}

impl SpirvReflectionGenerator for SpirvReflection {
    fn attributes(
        &self,
        names: &[&str],
    ) -> Result<VertexFormat, failure::Error> {

        if self.cache.is_none() {
            failure::bail!("Cache isn't constructed for shader: {:?}", self.stage());
        }

        // Fetch the layout indices of the string names
        assert!(names.len() < 64);
        let mut locations = smallvec::SmallVec::<[u32; 64]>::new();
        for name in names {
           locations.push(self.input_attributes.get(&name.to_string()).ok_or(failure::format_err!("Attribute named {} does not exist", name))?.location);
        }
        locations.sort();

        let mut stride: u32 = 0;
        let elements = locations.iter()
            .filter_map(|n| {
                let mut element = self.cache.as_ref().unwrap().vertices.0[*n as usize].clone();
                element.offset = stride;
                stride += element.format.surface_desc().bits as u32 / 8;
                return Some(element);
                None
            }).collect::<Vec<_>>();

        Ok(VertexFormat {
            attributes: std::borrow::Cow::from(elements),
            stride,
        })
    }

    fn attributes_range<R: std::ops::RangeBounds<usize>>(
        &self,
        range: R,
    ) -> Result<VertexFormat, failure::Error> {
        let cache = self.cache.as_ref().ok_or(failure::format_err!(
            "SpirvCachedGfxDescription not created for this reflection"
        ))?;

        let mut stride: u32 = 0;
        let elements = cache.vertices.0.iter().enumerate()
            .filter_map(|(n, e)| {
                if range_contains(&range, &n) {
                    let mut element = e.clone();
                    element.offset = stride;
                    stride += e.format.surface_desc().bits as u32 / 8;
                    return Some(element);
                }
                None
            }).collect::<Vec<_>>();


        Ok(VertexFormat {
            attributes: std::borrow::Cow::from(elements),
            stride,
        })
    }

    #[inline(always)]
    fn layout(&self) -> Result<Layout, failure::Error> {
        Ok(self
            .cache
            .as_ref()
            .ok_or(failure::format_err!(
                "SpirvCachedGfxDescription not created for this reflection"
            ))?
            .layout
            .clone())
    }

    #[inline(always)]
    fn stage(&self) -> ShaderStageFlags {
        self.stage_flag
    }

    #[inline(always)]
    fn push_constants(
        &self,
        range: Option<std::ops::Range<usize>>,
    ) -> Result<Vec<(ShaderStageFlags, std::ops::Range<u32>)>, failure::Error> {
        Ok(self.push_constants.clone())
    }
}

#[derive(Debug)]
pub struct ShaderStorage<B: Backend> {
    stage: ShaderStageFlags,
    spirv: Vec<u8>,
    reflection: SpirvReflection,
    module: Option<B::ShaderModule>,
}
impl<B: Backend> ShaderStorage<B> {
    pub fn get_entry_point<'a>(
        &'a self,
    ) -> Result<Option<gfx_hal::pso::EntryPoint<'a, B>>, failure::Error> {
        Ok(Some(gfx_hal::pso::EntryPoint {
            entry: &self.reflection.entrypoints.get(0).ok_or(failure::format_err!("Shader {:?} missing entry point", self.stage))?.1,
            module: self.module.as_ref().unwrap(),
            specialization: gfx_hal::pso::Specialization::default(),
        }))
    }

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

    fn manual_drop(&mut self, factory: &rendy_factory::Factory<B>) {
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
            panic!(
                "This shader storage class needs to be manually dropped with manual_drop() first"
            );
        }
    }
}

//    /// Build final set reflection
//    pub(crate) fn new(set_reflection: SpirvReflection, set_description: gfx_hal::pso::GraphicsShaderSet<'a, B>, shaders: Vec<(Vec<u8>, SpirvReflection)>) -> Self
#[derive(derivative::Derivative, Debug)]
#[derivative(Default(bound = ""))]
pub struct ShaderSet<B: Backend> {
    shaders: std::collections::HashMap<ShaderStageFlags, ShaderStorage<B>>,
    set_reflection: SpirvReflection,
}

impl<B: Backend> ShaderSet<B> {
    /// Returns gfx_hal graphics shader set representation (Multiple shader stages need to be provided)
    pub fn load(
        &mut self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<&mut Self, failure::Error> {
        for (_, v) in self.shaders.iter_mut() {
            unsafe { v.compile(factory)? }
        }

        Ok(self)
    }

    pub fn raw<'a>(
        &'a self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<(gfx_hal::pso::GraphicsShaderSet<'a, B>), failure::Error> {
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

    pub fn manual_drop(&mut self, factory: &rendy_factory::Factory<B>) {
        for (_, shader) in self.shaders.iter_mut() {
            shader.manual_drop(factory);
        }
    }
}

impl<B: Backend> SpirvReflectionGenerator for ShaderSet<B> {
    #[inline(always)]
    /// Returns attributes from range
    fn attributes_range<R: std::ops::RangeBounds<usize>>(
        &self,
        range: R,
    ) -> Result<VertexFormat, failure::Error> {
        self.set_reflection.attributes_range(range)
    }

    #[inline(always)]
    /// Returns attributes from range
    fn attributes(
        &self,
        names: &[&str],
    ) -> Result<VertexFormat, failure::Error> {
        self.set_reflection.attributes(names)
    }


    #[inline(always)]
    /// Returns from full set reflection
    fn layout(&self) -> Result<Layout, failure::Error> {
        self.set_reflection.layout()
    }

    #[inline(always)]
    fn stage(&self) -> ShaderStageFlags {
        self.set_reflection.stage()
    }

    #[inline(always)]
    fn push_constants(
        &self,
        range: Option<std::ops::Range<usize>>,
    ) -> Result<Vec<(ShaderStageFlags, std::ops::Range<u32>)>, failure::Error> {
        self.set_reflection.push_constants(range)
    }
}

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
    pub fn build<B: Backend>(
        &self,
        factory: &rendy_factory::Factory<B>,
    ) -> Result<ShaderSet<B>, failure::Error> {
        let mut set = ShaderSet::<B>::default();

        if self.vertex.is_none() && self.compute.is_none() {
            failure::bail!("A vertex or compute shader must be provided");
        }

        let vertex = self.vertex.clone().unwrap();
        let mut storage = ShaderStorage {
            stage: ShaderStageFlags::VERTEX,
            spirv: vertex.0,
            reflection: vertex.1,
            module: None,
        };
        unsafe {
            storage.compile(factory);
        }
        set.shaders.insert(ShaderStageFlags::VERTEX, storage);

        if let Some(fragment) = self.fragment.clone() {
            let mut storage = ShaderStorage {
                stage: ShaderStageFlags::FRAGMENT,
                spirv: fragment.0,
                reflection: fragment.1,
                module: None,
            };
            unsafe {
                storage.compile(factory);
            }
            set.shaders.insert(ShaderStageFlags::FRAGMENT, storage);
        }

        if let Some(hull) = self.hull.clone() {
            let mut storage = ShaderStorage {
                stage: ShaderStageFlags::HULL,
                spirv: hull.0,
                reflection: hull.1,
                module: None,
            };
            unsafe {
                storage.compile(factory);
            }
            set.shaders.insert(ShaderStageFlags::HULL, storage);
        }

        if let Some(geometry) = self.geometry.clone() {
            let mut storage = ShaderStorage {
                stage: ShaderStageFlags::GEOMETRY,
                spirv: geometry.0,
                reflection: geometry.1,
                module: None,
            };
            unsafe {
                storage.compile(factory);
            }
            set.shaders.insert(ShaderStageFlags::GEOMETRY, storage);
        }

        if let Some(domain) = self.domain.clone() {
            let mut storage = ShaderStorage {
                stage: ShaderStageFlags::DOMAIN,
                spirv: domain.0,
                reflection: domain.1,
                module: None,
            };
            unsafe {
                storage.compile(factory);
            }
            set.shaders.insert(ShaderStageFlags::DOMAIN, storage);
        }
        if let Some(compute) = self.compute.clone() {
            let mut storage = ShaderStorage {
                stage: ShaderStageFlags::COMPUTE,
                spirv: compute.0,
                reflection: compute.1,
                module: None,
            };
            unsafe {
                storage.compile(factory);
            }
            set.shaders.insert(ShaderStageFlags::COMPUTE, storage);
        }


        Ok(set)
    }

    pub fn reflect(mut self) -> Result<Self, failure::Error> {
        if self.vertex.is_none() && self.compute.is_none() {
            failure::bail!("A vertex or compute shader must be provided");
        }

        // We need to combine and merge all the reflections into a single SpirvReflection instance
        let mut reflections = Vec::new();
        if let Some(vertex) = self.vertex.as_mut() {
            vertex.1 = SpirvReflection::reflect(&vertex.0)?;
            reflections.push(&vertex.1);
        }
        if let Some(fragment) = self.fragment.as_mut() {
            fragment.1 = SpirvReflection::reflect(&fragment.0)?;
            reflections.push(&fragment.1);
        }
        if let Some(hull) = self.hull.as_mut() {
            hull.1 = SpirvReflection::reflect(&hull.0)?;
            reflections.push(&hull.1);
        }
        if let Some(domain) = self.domain.as_mut() {
            domain.1 = SpirvReflection::reflect(&domain.0)?;
            reflections.push(&domain.1);
        }
        if let Some(compute) = self.compute.as_mut() {
            compute.1 = SpirvReflection::reflect(&compute.0)?;
            reflections.push(&compute.1);
        }
        self.set_reflection = Some(merge(&reflections)?);
        self.set_reflection.as_mut().unwrap().compile_cache()?;
        log::trace!("Merged set reflection: {:?}", self.set_reflection);

        Ok(self)
    }

    #[inline(always)]
    fn reflect_shader(
        &mut self,
        data: std::borrow::Cow<'_, [u8]>,
    ) -> Result<SpirvReflection, failure::Error> {
        Ok(SpirvReflection::reflect(&data)?)
    }

    #[inline(always)]
    pub fn with_vertex<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.vertex = Some((data.to_vec(), self.reflect_shader(data)?));
        Ok(self)
    }

    #[inline(always)]
    pub fn with_fragment<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.fragment = Some((data.to_vec(), self.reflect_shader(data)?));
        Ok(self)
    }

    #[inline(always)]
    pub fn with_geometry<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.geometry = Some((data.to_vec(), self.reflect_shader(data)?));
        Ok(self)
    }

    #[inline(always)]
    pub fn with_hull<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.hull = Some((data.to_vec(), self.reflect_shader(data)?));
        Ok(self)
    }

    #[inline(always)]
    pub fn with_domain<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.domain = Some((data.to_vec(), self.reflect_shader(data)?));
        Ok(self)
    }

    #[inline(always)]
    pub fn with_compute<S: Shader>(mut self, shader: &S) -> Result<Self, failure::Error> {
        let data = shader.spirv()?;
        self.compute = Some((data.to_vec(), self.reflect_shader(data)?));
        Ok(self)
    }
}

impl SpirvReflectionGenerator for ShaderSetBuilder {
    #[inline(always)]
    fn attributes_range<R: std::ops::RangeBounds<usize>>(
        &self,
        range: R,
    ) -> Result<VertexFormat, failure::Error> {
        self.set_reflection.as_ref().ok_or(failure::format_err!("Attempting to fetch attributes without a set reflection. reflect() must be called first after completing a set build"))?.attributes_range(range)
    }

    /// Returns attributes from range
    fn attributes(
        &self,
        names: &[&str],
    ) -> Result<VertexFormat, failure::Error> {
        self.set_reflection.as_ref().ok_or(failure::format_err!("Attempting to fetch attributes without a set reflection. reflect() must be called first after completing a set build"))?.attributes(names)
    }

    #[inline(always)]
    fn layout(&self) -> Result<Layout, failure::Error> {
        self.set_reflection.as_ref().ok_or(failure::format_err!("Attempting to fetch layout without a set reflection. reflect() must be called first after completing a set build"))?.layout()
    }

    #[inline(always)]
    fn stage(&self) -> ShaderStageFlags {
        unimplemented!()
        //self.set_reflection.as_ref().ok_or(failure::format_err!("Attempting to fetch layout without a set reflection. reflect() must be called first after completing a set build"))?.stage()
    }

    #[inline(always)]
    fn push_constants(
        &self,
        range: Option<std::ops::Range<usize>>,
    ) -> Result<Vec<(ShaderStageFlags, std::ops::Range<u32>)>, failure::Error> {
        unimplemented!()
        //self.set_reflection.push_constants()
    }
}

fn merge(reflections: &[&SpirvReflection]) -> Result<SpirvReflection, failure::Error> {
    let mut descriptor_sets = Vec::<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>>::new();
    let mut set_push_constants = Vec::new();
    let mut set_stage_flags = ShaderStageFlags::empty();
    let mut set_entry_points = Vec::new();
    let mut input_attributes = HashMap::new();

    for s in reflections.iter() {
        let current_layout = &s.descriptor_sets;

        set_stage_flags.insert(s.stage());
        set_entry_points.extend(s.entrypoints.clone());
        set_push_constants.extend(s.push_constants(None)?);

        if s.stage() == ShaderStageFlags::VERTEX {
            input_attributes = s.input_attributes.clone();
        }

        for (n, set) in current_layout.iter().enumerate() {
            match descriptor_sets
                .get(n)
                .map(|existing| compare_set(set, existing))
            {
                None => descriptor_sets.push(set.clone()),
                Some(SetEquality::NotEqual) => {
                    return Err(failure::format_err!(
                        "Mismatching bindings between shaders for set #{}",
                        n
                    ));
                }
                Some(SetEquality::SupersetOf) => {
                    descriptor_sets[n] = set.clone(); // Overwrite it
                }
                Some(SetEquality::Equal) | Some(SetEquality::SubsetOf) => {
                    for binding in descriptor_sets[n].iter_mut() {
                        binding.stage_flags |= s.stage()
                    }
                } // We match, just skip it
            }
        }
    }

    SpirvReflection::new(
        set_stage_flags,
        set_entry_points,
        input_attributes,
        HashMap::new(),
        descriptor_sets,
        set_push_constants,
    )
}

/// This enum provides logical comparison results for descriptor sets. Because shaders can share bindings,
/// we cannot do a strict equality check for exclusion - we must see if shaders match, or if they are the same bindings
/// but mismatched descriptions.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BindingEquality {
    /// The bindings match
    Equal,
    /// The bindings share a binding index, but have different values. This is usually an error case.
    SameBindingNonEqual,
    /// The bindings do not equal
    NotEqual,
}

/// Logically compares two descriptor layout bindings to determine their relational equality.
pub fn compare_bindings(
    lhv: &gfx_hal::pso::DescriptorSetLayoutBinding,
    rhv: &gfx_hal::pso::DescriptorSetLayoutBinding,
) -> BindingEquality {
    if lhv.binding == rhv.binding
        && lhv.count == rhv.count
        && lhv.immutable_samplers == rhv.immutable_samplers
        && lhv.ty == rhv.ty
    {
        return BindingEquality::Equal;
    } else {
        if lhv.binding == rhv.binding {
            return BindingEquality::SameBindingNonEqual;
        }
    }

    return BindingEquality::NotEqual;
}

/// This enum provides logical comparison results for sets. Because shaders can share bindings,
/// we cannot do a strict equality check for exclusion - we must see if shaders match, or if they are the same bindings
/// but mismatched descriptions.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum SetEquality {
    /// The sets match
    Equal,
    /// The sets share a binding index, but have different values. This is usually an error case.
    SubsetOf,
    /// A superset set layout. This means LHV contains more data than RHV
    SupersetOf,
    /// Invalid Match
    NotEqual,
}

fn compare_set(
    lhv: &[gfx_hal::pso::DescriptorSetLayoutBinding],
    rhv: &[gfx_hal::pso::DescriptorSetLayoutBinding],
) -> SetEquality {
    use std::collections::HashMap;
    // Bindings may not be in order, so we need to make a copy and index them by binding.
    let mut lhv_bindings = HashMap::new();
    lhv.iter().for_each(|b| {
        lhv_bindings.insert(b.binding, b);
    });

    let mut rhv_bindings = HashMap::new();
    rhv.iter().for_each(|b| {
        rhv_bindings.insert(b.binding, b);
    });

    let predicate = if lhv.len() == rhv.len() {
        SetEquality::Equal
    } else if lhv.len() > rhv.len() {
        SetEquality::SupersetOf
    } else {
        SetEquality::SubsetOf
    };

    for (key, lhv_value) in lhv_bindings {
        if let Some(rhv_value) = rhv_bindings.get(&key) {
            match compare_bindings(lhv_value, rhv_value) {
                BindingEquality::Equal => {}
                BindingEquality::NotEqual | BindingEquality::SameBindingNonEqual => {
                    return SetEquality::NotEqual;
                }
            }
        } else {
            if predicate == SetEquality::Equal || predicate == SetEquality::SubsetOf {
                return SetEquality::NotEqual;
            }
        }
    }

    predicate
}

/// Function copied from range_contains RFC rust implementation in nightly
fn range_contains<U, R>(range: &R, item: &U) -> bool
    where
        U: ?Sized + PartialOrd<U>,
        R: RangeBounds<U>,
{
    (match range.start_bound() {
        Bound::Included(ref start) => *start <= item,
        Bound::Excluded(ref start) => *start < item,
        Bound::Unbounded => true,
    }) && (match range.end_bound() {
        Bound::Included(ref end) => item <= *end,
        Bound::Excluded(ref end) => item < *end,
        Bound::Unbounded => true,
    })
}