//!
//! Reflection extensions
//!
use crate::node::render::{Layout, SetLayout};
use crate::util::types::vertex::VertexFormat;
use rendy_shader::Shader;
use std::ops::{Bound, RangeBounds};

/// Extension for SpirvShaderReflection providing graph render type conversion
/// Implementors of this return the appropriate descriptor sets and attribute layers for a given shader set.
// this lives in graph instead of Shader due to not wanting to pull in all the layout requirements and cause a cross-dependency with rendy-shader
pub trait ShaderLayoutGenerator {
    /// Convert reflected descriptor sets to a Layout structure
    fn layout(&self) -> Result<Layout, failure::Error>;

    /// Convert reflected attributes to a direct gfx_hal element array
    fn attributes<B: RangeBounds<usize>>(&self, range: B) -> Result<VertexFormat, failure::Error>;

    /// Returns the stage flag for this shader
    fn stage(&self) -> Result<gfx_hal::pso::ShaderStageFlags, failure::Error>;

    /// Returns this shaders push constants in gfx-hal format
    fn push_constants(
        &self,
        range: Option<std::ops::Range<usize>>,
    ) -> Result<Vec<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>, failure::Error>;
}

/// This implementation lives to reflect a single shader description into a usable gfx layout
impl<S: Shader> ShaderLayoutGenerator for S {
    fn layout(&self) -> Result<Layout, failure::Error> {
        Ok(Layout {
            sets: self
                .reflect()?
                .descriptor_sets
                .iter()
                .map(|set| SetLayout {
                    bindings: set.clone(),
                })
                .collect(),
            push_constants: Vec::new(),
        })
    }

    fn attributes<B: RangeBounds<usize>>(&self, range: B) -> Result<VertexFormat, failure::Error> {
        let mut input_attributes = self.reflect()?.input_attributes.clone();

        let mut sizes = Vec::<u32>::with_capacity(input_attributes.len());
        sizes.resize(input_attributes.len(), u32::default());

        input_attributes.sort_by(|a, b| a.location.cmp(&b.location));

        input_attributes
            .iter()
            .filter(|e| e.location != 0xFFFFFFFF && range_contains(&range, &(e.location as usize)))
            .for_each(|e| {
                sizes.insert(
                    e.location as usize,
                    e.element.format.surface_desc().bits as u32 / 8,
                );
            });

        input_attributes
            .iter_mut()
            .enumerate()
            .filter(|(n, e)| e.location != 0xFFFFFFFF && range_contains(&range, n))
            .for_each(|(_, mut e)| {
                // Add the sizes before this element, and create its offset.
                let mut offset = 0;
                for i in 0..e.location {
                    offset += sizes.get(i as usize).unwrap();
                }
                e.element.offset = offset;
            });

        let elements: Vec<gfx_hal::pso::Element<gfx_hal::format::Format>> = input_attributes
            .iter()
            .enumerate()
            .filter(|(n, e)| e.location != 0xFFFFFFFF && range_contains(&range, n))
            .map(|(_, e)| e.element)
            .collect();

        let stride = sizes.iter().sum();
        log::trace!("Defining Vertex Buffer: {:?}, {:?}", elements, stride);

        Ok(VertexFormat {
            attributes: std::borrow::Cow::from(elements),
            stride,
        })
    }

    fn stage(&self) -> Result<gfx_hal::pso::ShaderStageFlags, failure::Error> {
        Ok(self.reflect()?.stage_flag)
    }

    fn push_constants(
        &self,
        range: Option<std::ops::Range<usize>>,
    ) -> Result<Vec<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>, failure::Error> {
        if range.is_some() {
            failure::bail!("We do not support manually specifying push constant ranges across shaders at this time")
        }

        Ok(self.reflect()?.push_constants.clone())
    }
}

/// Iterator function implementation for merging multiple shader layouts into a single shader set layout
pub trait SpirvLayoutMerger {
    /// Returns a merged Layout from the provided iterator
    fn merge(self) -> Result<Layout, failure::Error>;
}
impl<T> SpirvLayoutMerger for T
where
    T: IntoIterator,
    T::Item: Shader + Sized,
{
    fn merge(self) -> Result<Layout, failure::Error> {
        let mut sets = Vec::new();
        let mut push_constants = Vec::new();

        for s in self.into_iter() {
            let current_layout = s.layout()?;

            for (n, set) in current_layout.sets.iter().enumerate() {
                match sets.get(n).map(|existing| compare_set(set, existing)) {
                    None => sets.push(set.clone()),
                    Some(SetEquality::NotEqual) => {
                        return Err(failure::format_err!(
                            "Mismatching bindings between shaders for set #{}",
                            n
                        ));
                    }
                    Some(SetEquality::SupersetOf) => {
                        sets[n] = set.clone(); // Overwrite it
                    }
                    Some(SetEquality::Equal) | Some(SetEquality::SubsetOf) => {
                        for binding in sets[n].bindings.iter_mut() {
                            binding.stage_flags |= s.stage()?
                        }
                    } // We match, just skip it
                }
            }
            push_constants.extend(s.push_constants(None)?);
        }

        Ok(Layout {
            sets,
            push_constants,
        })
    }
}

/// Provides the ability to cache the reflected values so reflection does not occur again
#[derive(Clone, Debug)]
pub struct ShaderCache<'a> {
    attributes: VertexFormat<'a>,
    layout: Layout,
    stage: gfx_hal::pso::ShaderStageFlags,
    push_constants: Vec<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>,
}

impl<'a> ShaderLayoutGenerator for ShaderCache<'a> {
    fn layout(&self) -> Result<Layout, failure::Error> {
        Ok(self.layout.clone())
    }

    fn attributes<B: RangeBounds<usize>>(&self, range: B) -> Result<VertexFormat, failure::Error> {
        // Rebuild the VertexFormat based on the range given
        let mut stride: u32 = 0;
        let elements: Vec<_> = self
            .attributes
            .attributes
            .iter()
            .enumerate()
            .filter_map(|(n, e)| {
                if range_contains(&range, &n) {
                    stride += (e.format.surface_desc().bits / 8) as u32;
                    return Some(*e);
                }
                None
            })
            .collect();
        Ok(VertexFormat {
            attributes: std::borrow::Cow::from(elements),
            stride,
        })
    }

    fn stage(&self) -> Result<gfx_hal::pso::ShaderStageFlags, failure::Error> {
        Ok(self.stage)
    }

    fn push_constants(
        &self,
        range: Option<std::ops::Range<usize>>,
    ) -> Result<Vec<(gfx_hal::pso::ShaderStageFlags, std::ops::Range<u32>)>, failure::Error> {
        if let Some(range) = range {
            Ok(self.push_constants[range].to_vec())
        } else {
            Ok(self.push_constants.clone())
        }
    }
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

fn compare_set(lhv: &SetLayout, rhv: &SetLayout) -> SetEquality {
    use std::collections::HashMap;
    // Bindings may not be in order, so we need to make a copy and index them by binding.
    let mut lhv_bindings = HashMap::new();
    lhv.bindings.iter().for_each(|b| {
        lhv_bindings.insert(b.binding, b);
    });

    let mut rhv_bindings = HashMap::new();
    rhv.bindings.iter().for_each(|b| {
        rhv_bindings.insert(b.binding, b);
    });

    let predicate = if lhv.bindings.len() == rhv.bindings.len() {
        SetEquality::Equal
    } else if lhv.bindings.len() > rhv.bindings.len() {
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
