use gfx_hal::pso::ShaderStageFlags;
use rendy_util::types::{vertex::VertexFormat, Layout, SetLayout};
use spirv_reflect::ShaderModule;
use std::collections::HashMap;
use std::ops::{Bound, Range, RangeBounds};

pub(crate) mod types;
use types::*;

#[derive(Clone, Debug)]
pub(crate) struct SpirvCachedGfxDescription {
    pub vertices: (Vec<gfx_hal::pso::Element<gfx_hal::format::Format>>, u32),
    pub layout: Layout,
}

/// Contains intermediate structured data of reflected shader information.
#[derive(Clone, Debug)]
pub struct SpirvReflection {
    /// Vec of output variables with names.
    pub output_attributes: HashMap<(String, u8), gfx_hal::pso::AttributeDesc>,
    /// Vec of output variables with names.
    pub input_attributes: HashMap<(String, u8), gfx_hal::pso::AttributeDesc>,
    /// Hashmap of output variables with names.
    pub descriptor_sets: Vec<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>>,
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
        input_attributes: HashMap<(String, u8), gfx_hal::pso::AttributeDesc>,
        output_attributes: HashMap<(String, u8), gfx_hal::pso::AttributeDesc>,
        descriptor_sets: Vec<Vec<gfx_hal::pso::DescriptorSetLayoutBinding>>,
        push_constants: Vec<(ShaderStageFlags, Range<u32>)>,
    ) -> Result<Self, failure::Error> {
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

    pub(crate) fn compile_cache(mut self) -> Result<Self, failure::Error> {
        let mut stride: u32 = 0;
        let mut vertices = self
            .input_attributes
            .iter()
            .map(|(_, e)| e)
            .collect::<Vec<_>>();
        vertices.sort_by(|a, b| a.location.cmp(&b.location));

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

        self.cache = Some(SpirvCachedGfxDescription {
            vertices: (
                vertices
                    .iter()
                    .map(|e| {
                        let mut element = e.element.clone();
                        element.offset = stride;
                        stride += element.format.surface_desc().bits as u32 / 8;
                        element
                    })
                    .collect::<Vec<_>>(),
                stride,
            ),
            layout,
        });

        Ok(self)
    }

    /// This function performs the actual SPIRV reflection utilizing spirv-reflect-rs, and then converting it into appropriate structures which are then consumed by rendy.
    pub fn reflect(
        spirv: &[u8],
        entrypoint: Option<&str>,
    ) -> Result<SpirvReflection, failure::Error> {
        match ShaderModule::load_u8_data(spirv) {
            Ok(module) => {
                let stage_flag = convert_stage(module.get_shader_stage());

                let input_attributes =
                    generate_attributes(module.enumerate_input_variables(None).map_err(|e| {
                        failure::format_err!(
                            "Failed to get input attributes from spirv-reflect: {}",
                            e
                        )
                    })?);

                let output_attributes =
                    generate_attributes(module.enumerate_input_variables(None).map_err(|e| {
                        failure::format_err!(
                            "Failed to get output attributes from spirv-reflect: {}",
                            e
                        )
                    })?);

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
                    .map(|c| convert_push_constant(stage_flag, c))
                    .collect();

                let entrypoint = if let Some(e) = entrypoint { e } else { "main" };

                Self::new(
                    stage_flag,
                    Some(entrypoint.to_string()),
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

    /// Returns attributes based on their names in rendy/gfx_hal format in the form of a `VertexFormat`. Note that attributes are sorted in their layout location
    /// order, not in the order provided.
    pub fn attributes(&self, names: &[&str]) -> Result<VertexFormat, failure::Error> {
        if self.cache.is_none() {
            failure::bail!("Cache isn't constructed for shader: {:?}", self.stage());
        }

        // Fetch the layout indices of the string names
        assert!(names.len() < 64);
        let mut locations = smallvec::SmallVec::<[u32; 64]>::new();
        for name in names {
            // Does it contain an attribute (or array set) with this name?
            let interm = self
                .input_attributes
                .iter()
                .filter_map(|(k, v)| match k.0.eq_ignore_ascii_case(name) {
                    true => Some(v.location),
                    false => None,
                })
                .collect::<Vec<u32>>();

            if interm.len() < 1 {
                failure::bail!("Attribute named {} does not exist", name);
            }

            locations.extend_from_slice(&interm);
        }
        locations.sort();

        let mut stride: u32 = 0;
        let elements = locations
            .iter()
            .filter_map(|n| {
                let mut element = self.cache.as_ref().unwrap().vertices.0[*n as usize].clone();
                element.offset = stride;
                stride += element.format.surface_desc().bits as u32 / 8;
                return Some(element);
            })
            .collect::<Vec<_>>();

        Ok(VertexFormat {
            attributes: std::borrow::Cow::from(elements),
            stride,
        })
    }

    /// Returns attributes within a given index range in rendy/gfx_hal format in the form of a `VertexFormat`
    pub fn attributes_range<R: RangeBounds<usize>>(
        &self,
        range: R,
    ) -> Result<VertexFormat, failure::Error> {
        let cache = self.cache.as_ref().ok_or(failure::format_err!(
            "SpirvCachedGfxDescription not created for this reflection"
        ))?;

        let mut stride: u32 = 0;
        let elements = cache
            .vertices
            .0
            .iter()
            .enumerate()
            .filter_map(|(n, e)| {
                if range_contains(&range, &n) {
                    let mut element = e.clone();
                    element.offset = stride;
                    stride += e.format.surface_desc().bits as u32 / 8;
                    return Some(element);
                }
                None
            })
            .collect::<Vec<_>>();

        Ok(VertexFormat {
            attributes: std::borrow::Cow::from(elements),
            stride,
        })
    }

    /// Returns the merged descriptor set layouts of all shaders in this set in gfx_hal format in the form of a `Layout` structure.
    #[inline(always)]
    pub fn layout(&self) -> Result<Layout, failure::Error> {
        Ok(self
            .cache
            .as_ref()
            .ok_or(failure::format_err!(
                "SpirvCachedGfxDescription not created for this reflection"
            ))?
            .layout
            .clone())
    }

    /// Returns the combined stages of shaders which are in this set in the form of a `ShaderStageFlags` bitflag.
    #[inline]
    pub fn stage(&self) -> ShaderStageFlags {
        self.stage_flag
    }

    /// Returns the reflected push constants of this shader set in gfx_hal format.
    #[inline]
    pub fn push_constants(
        &self,
        range: Option<Range<usize>>,
    ) -> Result<Vec<(ShaderStageFlags, Range<u32>)>, failure::Error> {
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

pub(crate) fn merge(reflections: &[SpirvReflection]) -> Result<SpirvReflection, failure::Error> {
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
