use spirv_reflect::types::ReflectShaderStageFlags;
use rendy_core::hal::pso::ShaderStageFlags;

const NUM_STAGE_SLOTS: usize = 8;

/// An enum referring to a single shader stage in the graphics pipeline
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum ShaderStage {
    Vertex = 0,
    Hull = 1,
    Domain = 2,
    Geometry = 3,
    Fragment = 4,
    Compute = 5,
    Task = 6,
    Mesh = 7,
}
impl ShaderStage {
    /// Checks if the ShaderStage is compatible with a shader that is reflected
    /// upon.
    pub fn compatible_with_reflect(&self, reflect_stage: ReflectShaderStageFlags) -> bool {
        match self {
            Self::Vertex => reflect_stage.contains(ReflectShaderStageFlags::VERTEX),
            Self::Hull => reflect_stage.contains(ReflectShaderStageFlags::TESSELLATION_CONTROL),
            Self::Domain => reflect_stage.contains(ReflectShaderStageFlags::TESSELLATION_EVALUATION),
            Self::Geometry => reflect_stage.contains(ReflectShaderStageFlags::GEOMETRY),
            Self::Fragment => reflect_stage.contains(ReflectShaderStageFlags::FRAGMENT),
            Self::Compute => reflect_stage.contains(ReflectShaderStageFlags::COMPUTE),
            _ => false,
        }
    }
}
impl Into<ShaderStageFlags> for ShaderStage {
    fn into(self) -> ShaderStageFlags {
        match self {
            Self::Vertex => ShaderStageFlags::VERTEX,
            Self::Hull => ShaderStageFlags::HULL,
            Self::Domain => ShaderStageFlags::DOMAIN,
            Self::Geometry => ShaderStageFlags::GEOMETRY,
            Self::Fragment => ShaderStageFlags::FRAGMENT,
            Self::Compute => ShaderStageFlags::COMPUTE,
            Self::Task => ShaderStageFlags::TASK,
            Self::Mesh => ShaderStageFlags::MESH,
        }
    }
}
impl ShaderStage {
    /// Given an index referring to the integer repr of a stage variant, will
    /// return the ShaderStage.
    ///
    /// If invalid, will panic.
    pub fn from_index(index: u32) -> Self {
        match index {
            0 => Self::Vertex,
            1 => Self::Hull,
            2 => Self::Domain,
            3 => Self::Geometry,
            4 => Self::Fragment,
            5 => Self::Compute,
            6 => Self::Task,
            7 => Self::Mesh,
            _ => unreachable!(),
        }
    }

    /// Given a ShaderStageFlags with a single bit set, will return the
    /// equivallent ShaderStage variant.
    pub fn from_mask(mask: ShaderStageFlags) -> Option<Self> {
        match mask {
            ShaderStageFlags::VERTEX => Some(Self::Vertex),
            ShaderStageFlags::HULL => Some(Self::Hull),
            ShaderStageFlags::DOMAIN => Some(Self::Domain),
            ShaderStageFlags::GEOMETRY => Some(Self::Geometry),
            ShaderStageFlags::FRAGMENT => Some(Self::Fragment),
            ShaderStageFlags::COMPUTE => Some(Self::Compute),
            ShaderStageFlags::TASK => Some(Self::Task),
            ShaderStageFlags::MESH => Some(Self::Mesh),
            _ => None,
        }
    }
}

/// A map of ShaderStage to an optional inner type
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct StageMap<T> {
    slots: [Option<T>; NUM_STAGE_SLOTS],
}

impl<T> Default for StageMap<T> {
    fn default() -> Self {
        StageMap::new()
    }
}

impl<T> StageMap<T> {
    /// Creates an empty StageMap
    pub fn new() -> Self {
        Self {
            slots: [None, None, None, None, None, None, None, None],
        }
    }

    /// Inserts a new entry into the StageMap. If the element already exists,
    /// the old one will be returned.
    #[inline(always)]
    pub fn insert(&mut self, stage: ShaderStage, value: T) -> Option<T> {
        let mut elem = Some(value);
        std::mem::swap(&mut self.slots[stage as usize], &mut elem);
        elem
    }

    /// Removes an entry from the map and returns it if it exists.
    #[inline(always)]
    pub fn remove(&mut self, stage: ShaderStage) -> Option<T> {
        self.slots[stage as usize].take()
    }

    /// Returns an entry from the map if it exists
    #[inline(always)]
    pub fn get(&self, stage: ShaderStage) -> Option<&T> {
        self.slots[stage as usize].as_ref()
    }

    /// Iterates over all the existing elements in the map
    pub fn iter(&self) -> impl Iterator<Item = (ShaderStage, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(idx, v)| {
                v.as_ref().map(|v| (ShaderStage::from_index(idx as u32), v))
            })
    }

    /// Mutably iterates over all the existing elements in the map
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ShaderStage, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(idx, v)| {
                v.as_mut().map(|v| (ShaderStage::from_index(idx as u32), v))
            })
    }

    /// Iterates over all the elements in the map
    pub fn iter_all(&self) -> impl Iterator<Item = (ShaderStage, Option<&T>)> {
        self.slots
            .iter()
            .enumerate()
            .map(|(idx, v)| {
                (ShaderStage::from_index(idx as u32), v.as_ref())
            })
    }
}
