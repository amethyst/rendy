use rendy_core::hal::pso::ShaderStageFlags;
use rendy_core::types::{vertex::VertexFormat, Layout, SetLayout};
use spirv_reflect::ShaderModule;
use std::collections::HashMap;
use std::ops::{Bound, Range, RangeBounds};

pub(crate) mod types;
pub use types::ReflectTypeError;
use types::*;

/// The item kind that couldn't be retrieved from spirv-reflect.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RetrievalKind {
    /// Input attributes.
    InputAttrib,
    /// Output attributes.
    OutputAttrib,
    /// Descriptor sets.
    DescriptorSets,
    /// Push constants.
    PushConstants,
}

impl RetrievalKind {
    fn as_str(&self) -> &'static str {
        match *self {
            RetrievalKind::InputAttrib => "input attributes",
            RetrievalKind::OutputAttrib => "output attributes",
            RetrievalKind::DescriptorSets => "descriptor sets",
            RetrievalKind::PushConstants => "push constants",
        }
    }
}

/// A reflection error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReflectError {
    /// An item could not be retrieved from spirv-reflect.
    Retrieval(RetrievalKind, String),
    /// A spirv-reflect error occured.
    General(String),
    /// An attribute by the given name does not exist.
    NameDoesNotExist(String),
    /// The cache wasn't constructed for the shader.
    CacheNotConstructued(ShaderStageFlags),
    /// The bindings between the shaders of a set did not match.
    BindingsMismatch(usize),
    /// The SpirvCachedGfxDescription was not created.
    SpirvCachedGfxDescription,
    /// An error occured while reflecting a type.
    Type(ReflectTypeError),
    /// Neither a vertex nor a compute shader has been provided.
    NoVertComputeProvided,
}

impl std::error::Error for ReflectError {}
impl std::fmt::Display for ReflectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReflectError::Retrieval(kind, msg) => write!(
                f,
                "failed to get {} from spirv-reflect: {}",
                kind.as_str(),
                msg
            ),
            ReflectError::General(msg) => write!(f, "{}", msg),
            ReflectError::NameDoesNotExist(name) => {
                write!(f, "attribute named {} does not exist", name)
            }
            ReflectError::CacheNotConstructued(flags) => {
                write!(f, "cache isn't constructed for shader: {:?}", flags)
            }
            ReflectError::BindingsMismatch(set) => {
                write!(f, "mismatching bindings between shaders for set {}", set)
            }
            ReflectError::SpirvCachedGfxDescription => write!(
                f,
                "SpirvCachedGfxDescription not created for this reflection"
            ),
            ReflectError::Type(e) => write!(f, "{}", e),
            ReflectError::NoVertComputeProvided => {
                write!(f, "a vertex or compute shader must be provided")
            }
        }
    }
}

impl From<ReflectTypeError> for ReflectError {
    fn from(e: ReflectTypeError) -> Self {
        ReflectError::Type(e)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SpirvCachedGfxDescription {
    pub vertices: Vec<(u32, String, u8, rendy_core::hal::format::Format)>,
    pub layout: Layout,
}

/// Contains intermediate structured data of reflected shader information.
#[derive(Clone, Debug)]
pub struct SpirvReflection {
    /// Vec of output variables with names.
    pub output_attributes: HashMap<(String, u8), rendy_core::hal::pso::AttributeDesc>,
    /// Vec of output variables with names.
    pub input_attributes: HashMap<(String, u8), rendy_core::hal::pso::AttributeDesc>,
    /// Hashmap of output variables with names.
    pub descriptor_sets: Vec<Vec<rendy_core::hal::pso::DescriptorSetLayoutBinding>>,
    /// Stage flag of this shader
    pub stage_flag: ShaderStageFlags,
    /// Push Constants
    pub push_constants: Vec<(ShaderStageFlags, Range<u32>)>,
    /// All possible entrypoints to this shader
    pub entrypoints: Vec<(ShaderStageFlags, String)>,
    /// User selected entry point or default
    pub entrypoint: Option<String>,
    /// Cached value of gfx-hal specific data
    pub(crate) cache: Option<SpirvCachedGfxDescription>,
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
            entrypoint: None,
            cache: None,
        }
    }
}

impl SpirvReflection {
    pub(crate) fn new(
        stage_flag: ShaderStageFlags,
        entrypoint: Option<String>,
        entrypoints: Vec<(ShaderStageFlags, String)>,
        input_attributes: HashMap<(String, u8), rendy_core::hal::pso::AttributeDesc>,
        output_attributes: HashMap<(String, u8), rendy_core::hal::pso::AttributeDesc>,
        descriptor_sets: Vec<Vec<rendy_core::hal::pso::DescriptorSetLayoutBinding>>,
        push_constants: Vec<(ShaderStageFlags, Range<u32>)>,
    ) -> Result<Self, ReflectError> {
        Ok(SpirvReflection {
            output_attributes,
            input_attributes,
            descriptor_sets,
            stage_flag,
            push_constants,
            entrypoints,
            entrypoint: entrypoint,
            cache: None,
        })
    }

    pub(crate) fn compile_cache(mut self) -> Result<Self, ReflectError> {
        // BBreak apart the sets into the appropriate grouping

        let layout = if self.descriptor_sets.len() > 0 {
            Layout {
                sets: self
                    .descriptor_sets
                    .iter()
                    .map(|e| SetLayout {
                        bindings: e.clone(),
                    })
                    .collect(),
                push_constants: self.push_constants.clone(),
            }
        } else {
            Layout {
                sets: vec![],
                push_constants: self.push_constants.clone(),
            }
        };

        let mut vertices = self
            .input_attributes
            .iter()
            .map(|(k, e)| (e.location, k.0.clone(), k.1, e.element.format))
            .collect::<Vec<_>>();
        vertices.sort_by(|a, b| a.0.cmp(&b.0));

        self.cache = Some(SpirvCachedGfxDescription { vertices, layout });

        Ok(self)
    }

    /// This function performs the actual SPIRV reflection utilizing spirv-reflect-rs, and then converting it into appropriate structures which are then consumed by rendy.
    pub fn reflect(
        spirv: &[u32],
        entrypoint: Option<&str>,
    ) -> Result<SpirvReflection, ReflectError> {
        match ShaderModule::load_u32_data(spirv) {
            Ok(module) => {
                let stage_flag = convert_stage(module.get_shader_stage());

                let input_attributes =
                    generate_attributes(module.enumerate_input_variables(None).map_err(|e| {
                        ReflectError::Retrieval(RetrievalKind::InputAttrib, e.to_string())
                    })?);

                let output_attributes =
                    generate_attributes(module.enumerate_input_variables(None).map_err(|e| {
                        ReflectError::Retrieval(RetrievalKind::OutputAttrib, e.to_string())
                    })?);

                let descriptor_sets: Result<Vec<_>, _> = module
                    .enumerate_descriptor_sets(None)
                    .map_err(|e| {
                        ReflectError::Retrieval(RetrievalKind::DescriptorSets, e.to_string())
                    })?
                    .iter()
                    .map(ReflectInto::<Vec<rendy_core::hal::pso::DescriptorSetLayoutBinding>>::reflect_into)
                    .collect();

                // This is a fixup-step required because of our implementation. Because we dont pass the module around
                // to the all the reflect_into API's, we need to fix up the shader stage here at the end. Kinda a hack
                let mut descriptor_sets_final = descriptor_sets?;
                descriptor_sets_final.iter_mut().for_each(|v| {
                    v.iter_mut()
                        .for_each(|mut set| set.stage_flags = stage_flag);
                });

                let push_constants: Result<Vec<_>, _> = module
                    .enumerate_push_constant_blocks(None)
                    .map_err(|e| {
                        ReflectError::Retrieval(RetrievalKind::PushConstants, e.to_string())
                    })?
                    .iter()
                    .map(|c| convert_push_constant(stage_flag, c))
                    .collect();

                let entrypoint = if let Some(e) = entrypoint { e } else { "main" };

                Self::new(
                    stage_flag,
                    Some(entrypoint.to_string()),
                    vec![(stage_flag, module.get_entry_point_name())],
                    input_attributes.map_err(|e| {
                        ReflectError::Retrieval(RetrievalKind::InputAttrib, e.to_string())
                    })?,
                    output_attributes.map_err(|e| {
                        ReflectError::Retrieval(RetrievalKind::OutputAttrib, e.to_string())
                    })?,
                    descriptor_sets_final,
                    push_constants?,
                )
            }
            Err(e) => return Err(ReflectError::General(e.to_string())),
        }
    }

    /// Returns attributes based on their names in rendy/rendy_core::hal format in the form of a `VertexFormat`. Note that attributes are sorted in their layout location
    /// order, not in the order provided.
    pub fn attributes(&self, names: &[&str]) -> Result<VertexFormat, ReflectError> {
        let cache = self
            .cache
            .as_ref()
            .ok_or(ReflectError::CacheNotConstructued(self.stage()))?;
        let mut attributes = smallvec::SmallVec::<[_; 64]>::new();

        for name in names {
            // Does it contain an attribute (or array set) with this name?
            let this_name_attributes = cache
                .vertices
                .iter()
                .filter(|(_, vert_name, _, _)| name.eq_ignore_ascii_case(vert_name))
                .cloned();
            let before = attributes.len();
            attributes.extend(this_name_attributes);
            if attributes.len() == before {
                return Err(ReflectError::NameDoesNotExist(name.to_string()));
            }
        }
        attributes.sort_by_key(|a| a.0);

        Ok(VertexFormat::new(
            attributes
                .into_iter()
                .map(|(_, name, _, format)| (format, name))
                .collect::<Vec<_>>(),
        ))
    }

    /// Returns attributes within a given index range in rendy/rendy_core::hal format in the form of a `VertexFormat`
    pub fn attributes_range<R: RangeBounds<u32>>(
        &self,
        range: R,
    ) -> Result<VertexFormat, ReflectError> {
        let cache = self
            .cache
            .as_ref()
            .ok_or(ReflectError::CacheNotConstructued(self.stage()))?;

        let attributes = cache
            .vertices
            .iter()
            .filter(|(loc, _, _, _)| range_contains(&range, loc))
            .map(|(_, name, _, format)| (*format, name.clone()))
            .collect::<Vec<_>>();

        Ok(VertexFormat::new(attributes))
    }

    /// Returns the merged descriptor set layouts of all shaders in this set in rendy_core::hal format in the form of a `Layout` structure.
    #[inline(always)]
    pub fn layout(&self) -> Result<Layout, ReflectError> {
        Ok(self
            .cache
            .as_ref()
            .ok_or(ReflectError::SpirvCachedGfxDescription)?
            .layout
            .clone())
    }

    /// Returns the combined stages of shaders which are in this set in the form of a `ShaderStageFlags` bitflag.
    #[inline]
    pub fn stage(&self) -> ShaderStageFlags {
        self.stage_flag
    }

    /// Returns the reflected push constants of this shader set in rendy_core::hal format.
    #[inline]
    pub fn push_constants(
        &self,
        range: Option<Range<usize>>,
    ) -> Result<Vec<(ShaderStageFlags, Range<u32>)>, ReflectError> {
        if range.is_some() {
            Ok(self
                .push_constants
                .iter()
                .enumerate()
                .filter_map(|(n, p)| {
                    if range_contains(range.as_ref().unwrap(), &n) {
                        return Some(p.clone());
                    }
                    return None;
                })
                .collect())
        } else {
            Ok(self.push_constants.clone())
        }
    }
}

pub(crate) fn merge(reflections: &[SpirvReflection]) -> Result<SpirvReflection, ReflectError> {
    let mut descriptor_sets = Vec::<Vec<rendy_core::hal::pso::DescriptorSetLayoutBinding>>::new();
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
                    return Err(ReflectError::BindingsMismatch(n));
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
        None,
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
    lhv: &rendy_core::hal::pso::DescriptorSetLayoutBinding,
    rhv: &rendy_core::hal::pso::DescriptorSetLayoutBinding,
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
    lhv: &[rendy_core::hal::pso::DescriptorSetLayoutBinding],
    rhv: &[rendy_core::hal::pso::DescriptorSetLayoutBinding],
) -> SetEquality {
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
