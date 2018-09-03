//! Capability module docs.

use chain::stage::PipelineStageFlags;


bitflags! {
    /// Bitmask specifying capabilities of queues in a queue family.
    /// See Vulkan docs for detailed info:
    /// https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkQueueFlagBits.html
    #[repr(transparent)]
    pub struct CapabilityFlags: u32 {
        /// Queues from families with this capability flag set are able to perform graphics commands.
        const GRAPHICS = 0x00000001;

        /// Queues from families with this capability flag set are able to perform compute commands.
        const COMPUTE = 0x00000002;

        /// Queues from families with this capability flag set are able to perform transfer commands.
        const TRANSFER = 0x00000004;

        /// ???
        const SPARSE_BINDING = 0x00000008;

        /// ???
        const PROTECTED = 0x00000010;
    }
}

/// Capable of transfer only.
#[derive(Clone, Copy, Debug)]
pub struct Transfer;

/// Capable of either compute or graphics commands execution.
#[derive(Clone, Copy, Debug)]
pub struct Execute;

/// Capable of compute commands execution.
#[derive(Clone, Copy, Debug)]
pub struct Compute;

/// Capable of graphics command execution.
#[derive(Clone, Copy, Debug)]
pub struct Graphics;

/// Capable of any commands execution.
#[derive(Clone, Copy, Debug)]
pub struct General;

/// Abstract capability specifier.
pub trait Capability: Copy {
    /// Try to create capability instance from flags.
    /// Instance will be created if all required flags set.
    fn from_flags(flags: CapabilityFlags) -> Option<Self>;
}

impl Capability for CapabilityFlags {
    fn from_flags(flags: CapabilityFlags) -> Option<Self> {
        Some(flags)
    }
}

impl Capability for Transfer {
    fn from_flags(flags: CapabilityFlags) -> Option<Self> {
        if flags.contains(CapabilityFlags::TRANSFER) {
            Some(Transfer)
        } else {
            None
        }
    }
}

impl Capability for Execute {
    fn from_flags(flags: CapabilityFlags) -> Option<Self> {
        if flags.intersects(CapabilityFlags::COMPUTE | CapabilityFlags::GRAPHICS) {
            Some(Execute)
        } else {
            None
        }
    }
}

impl Capability for Compute {
    fn from_flags(flags: CapabilityFlags) -> Option<Self> {
        if flags.contains(CapabilityFlags::COMPUTE) {
            Some(Compute)
        } else {
            None
        }
    }
}

impl Capability for Graphics {
    fn from_flags(flags: CapabilityFlags) -> Option<Self> {
        if flags.contains(CapabilityFlags::GRAPHICS) {
            Some(Graphics)
        } else {
            None
        }
    }
}

impl Capability for General {
    fn from_flags(flags: CapabilityFlags) -> Option<Self> {
        if flags.contains(CapabilityFlags::GRAPHICS | CapabilityFlags::COMPUTE) {
            Some(General)
        } else {
            None
        }
    }
}

/// Get capabilities required by pipeline stages.
pub fn required_queue_capability(stages: PipelineStageFlags) -> CapabilityFlags {
    let mut capability = CapabilityFlags::empty();
    if stages.contains(PipelineStageFlags::DRAW_INDIRECT) { capability |= CapabilityFlags::GRAPHICS | CapabilityFlags::COMPUTE; }
    if stages.contains(PipelineStageFlags::VERTEX_INPUT) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::VERTEX_SHADER) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::TESSELLATION_CONTROL_SHADER) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::TESSELLATION_EVALUATION_SHADER) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::GEOMETRY_SHADER) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::FRAGMENT_SHADER) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::EARLY_FRAGMENT_TESTS) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::LATE_FRAGMENT_TESTS) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT) { capability |= CapabilityFlags::GRAPHICS; }
    if stages.contains(PipelineStageFlags::COMPUTE_SHADER) { capability |= CapabilityFlags::COMPUTE; }
    if stages.contains(PipelineStageFlags::TRANSFER) { capability |= CapabilityFlags::GRAPHICS | CapabilityFlags::COMPUTE | CapabilityFlags::TRANSFER; }
    if stages.contains(PipelineStageFlags::ALL_GRAPHICS) { capability |= CapabilityFlags::GRAPHICS; }
    capability
}

