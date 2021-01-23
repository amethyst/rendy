//! Types shared across rendy
pub mod vertex;

/// Set layout
#[derive(Clone, Debug, Default)]
pub struct SetLayout {
    /// Set layout bindings.
    pub bindings: Vec<crate::hal::pso::DescriptorSetLayoutBinding>,
}

/// Pipeline layout
#[derive(Clone, Debug)]
pub struct Layout {
    /// Sets in pipeline layout.
    pub sets: Vec<SetLayout>,

    /// Push constants in pipeline layout.
    pub push_constants: Vec<(crate::hal::pso::ShaderStageFlags, std::ops::Range<u32>)>,
}
