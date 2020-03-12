use fxhash::{FxBuildHasher, FxHashMap};
pub use gfx_hal::pso::{
    BufferDescriptorFormat, BufferDescriptorType, DescriptorRangeDesc, DescriptorSetLayoutBinding,
    DescriptorType, ImageDescriptorType,
};
use std::{
    cmp::Ordering,
    ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign},
};

/// Number of descriptors per type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptorRanges {
    counts: FxHashMap<DescriptorType, u32>,
}
impl std::hash::Hash for DescriptorRanges {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.iter().for_each(|desc| {
            desc.ty.hash(state);
            desc.count.hash(state);
        })
    }
}

impl DescriptorRanges {
    /// Create new instance without descriptors.
    pub fn zero() -> Self {
        DescriptorRanges {
            counts: FxHashMap::with_capacity_and_hasher(12, FxBuildHasher::default()),
        }
    }

    /// Add a single layout binding.
    /// Useful when created with `DescriptorRanges::zero()`.
    pub fn add_binding(&mut self, binding: DescriptorSetLayoutBinding) {
        *self.counts.entry(binding.ty).or_insert_with(|| 1) += binding.count as u32;
    }

    /// Iterate through ranges yelding
    /// descriptor types and their amount.
    pub fn iter(&self) -> DescriptorRangesIter<'_> {
        DescriptorRangesIter {
            iter: self.counts.iter(),
        }
    }

    /// Calculate ranges from bindings.
    pub fn from_bindings(bindings: &[DescriptorSetLayoutBinding]) -> Self {
        let mut descs = Self::zero();

        bindings.iter().for_each(|binding| {
            descs.add_binding(binding.clone());
        });

        descs
    }

    /// Calculate ranges from bindings, specified with an iterator.
    pub fn from_binding_iter<I>(bindings: I) -> Self
    where
        I: Iterator<Item = DescriptorSetLayoutBinding>,
    {
        let mut descs = Self::zero();

        bindings.for_each(|binding| {
            descs.add_binding(binding);
        });

        descs
    }
}

/*
impl PartialOrd for DescriptorRanges {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let mut ord = self.counts[0].partial_cmp(&other.counts[0])?;
        for i in 1..DESCRIPTOR_TYPES_COUNT {
            match (ord, self.counts[i].partial_cmp(&other.counts[i])?) {
                (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => {
                    return None;
                }
                (Ordering::Equal, new) => ord = new,
                _ => (),
            }
        }
        Some(ord)
    }
}
*/

impl Add for DescriptorRanges {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign for DescriptorRanges {
    fn add_assign(&mut self, rhs: Self) {
        rhs.counts.iter().for_each(|(ty, count)| {
            *self.counts.entry(*ty).or_insert_with(|| 1) += *count;
        });
    }
}

impl Sub for DescriptorRanges {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl SubAssign for DescriptorRanges {
    fn sub_assign(&mut self, rhs: Self) {
        rhs.counts.iter().for_each(|(ty, rhv_count)| {
            self.counts.get_mut(&ty).map(|count| *count -= *rhv_count);
        });
    }
}

impl Mul<u32> for DescriptorRanges {
    type Output = Self;
    fn mul(mut self, rhs: u32) -> Self {
        self *= rhs;
        self
    }
}

impl MulAssign<u32> for DescriptorRanges {
    fn mul_assign(&mut self, rhs: u32) {
        self.counts.iter_mut().for_each(|(_, count)| *count *= rhs);
    }
}

impl<'a> IntoIterator for &'a DescriptorRanges {
    type Item = DescriptorRangeDesc;
    type IntoIter = DescriptorRangesIter<'a>;

    fn into_iter(self) -> DescriptorRangesIter<'a> {
        self.iter()
    }
}

/// Iterator over descriptor ranges.
pub struct DescriptorRangesIter<'a> {
    iter: std::collections::hash_map::Iter<'a, DescriptorType, u32>,
}

impl<'a> Iterator for DescriptorRangesIter<'a> {
    type Item = DescriptorRangeDesc;

    fn next(&mut self) -> Option<DescriptorRangeDesc> {
        self.iter.next().map(|(ty, count)| DescriptorRangeDesc {
            ty: *ty,
            count: *count as usize,
        })
    }
}
