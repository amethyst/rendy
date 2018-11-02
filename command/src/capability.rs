//! Capability module docs.

use ash::vk;

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
    fn from_flags(flags: vk::QueueFlags) -> Option<Self>;

    /// Convert into `vk::QueueFlags`
    fn into_flags(self) -> vk::QueueFlags;
}

impl Capability for vk::QueueFlags {
    fn from_flags(flags: vk::QueueFlags) -> Option<Self> {
        Some(flags)
    }

    fn into_flags(self) -> vk::QueueFlags {
        self
    }
}

impl Capability for Transfer {
    fn from_flags(flags: vk::QueueFlags) -> Option<Self> {
        if flags.subset(vk::QueueFlags::TRANSFER) {
            Some(Transfer)
        } else {
            None
        }
    }

    fn into_flags(self) -> vk::QueueFlags {
        vk::QueueFlags::TRANSFER
    }
}

impl Capability for Execute {
    fn from_flags(flags: vk::QueueFlags) -> Option<Self> {
        if flags.intersects(vk::QueueFlags::COMPUTE | vk::QueueFlags::GRAPHICS) {
            Some(Execute)
        } else {
            None
        }
    }

    fn into_flags(self) -> vk::QueueFlags {
        vk::QueueFlags::COMPUTE | vk::QueueFlags::GRAPHICS
    }
}

impl Capability for Compute {
    fn from_flags(flags: vk::QueueFlags) -> Option<Self> {
        if flags.subset(vk::QueueFlags::COMPUTE) {
            Some(Compute)
        } else {
            None
        }
    }

    fn into_flags(self) -> vk::QueueFlags {
        vk::QueueFlags::COMPUTE
    }
}

impl Capability for Graphics {
    fn from_flags(flags: vk::QueueFlags) -> Option<Self> {
        if flags.subset(vk::QueueFlags::GRAPHICS) {
            Some(Graphics)
        } else {
            None
        }
    }

    fn into_flags(self) -> vk::QueueFlags {
        vk::QueueFlags::GRAPHICS
    }
}

impl Capability for General {
    fn from_flags(flags: vk::QueueFlags) -> Option<Self> {
        if flags.subset(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE) {
            Some(General)
        } else {
            None
        }
    }

    fn into_flags(self) -> vk::QueueFlags {
        vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE
    }
}

/// Check if capability supported.
pub trait Supports<C>: Capability {
    /// Check runtime capability.
    fn supports(&self) -> Option<C>;
}

impl Supports<Transfer> for Transfer {
    fn supports(&self) -> Option<Transfer> {
        Some(Transfer)
    }
}

impl Supports<Transfer> for Compute {
    fn supports(&self) -> Option<Transfer> {
        Some(Transfer)
    }
}

impl Supports<Transfer> for Graphics {
    fn supports(&self) -> Option<Transfer> {
        Some(Transfer)
    }
}

impl Supports<Transfer> for General {
    fn supports(&self) -> Option<Transfer> {
        Some(Transfer)
    }
}

impl Supports<Execute> for Compute {
    fn supports(&self) -> Option<Execute> {
        Some(Execute)
    }
}

impl Supports<Execute> for Graphics {
    fn supports(&self) -> Option<Execute> {
        Some(Execute)
    }
}

impl Supports<Execute> for General {
    fn supports(&self) -> Option<Execute> {
        Some(Execute)
    }
}

impl Supports<Compute> for Compute {
    fn supports(&self) -> Option<Compute> {
        Some(Compute)
    }
}

impl Supports<Compute> for General {
    fn supports(&self) -> Option<Compute> {
        Some(Compute)
    }
}

impl Supports<Graphics> for Graphics {
    fn supports(&self) -> Option<Graphics> {
        Some(Graphics)
    }
}

impl Supports<Graphics> for General {
    fn supports(&self) -> Option<Graphics> {
        Some(Graphics)
    }
}

impl Supports<Transfer> for vk::QueueFlags {
    fn supports(&self) -> Option<Transfer> {
        Transfer::from_flags(*self)
    }
}

impl Supports<Execute> for vk::QueueFlags {
    fn supports(&self) -> Option<Execute> {
        Execute::from_flags(*self)
    }
}

impl Supports<Compute> for vk::QueueFlags {
    fn supports(&self) -> Option<Compute> {
        Compute::from_flags(*self)
    }
}

impl Supports<Graphics> for vk::QueueFlags {
    fn supports(&self) -> Option<Graphics> {
        Graphics::from_flags(*self)
    }
}

/// Get capabilities required by pipeline stages.
pub fn required_queue_capability(stages: vk::PipelineStageFlags) -> vk::QueueFlags {
    let mut capability = vk::QueueFlags::empty();
    if stages.subset(vk::PipelineStageFlags::DRAW_INDIRECT) {
        capability |= vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE;
    }
    if stages.subset(vk::PipelineStageFlags::VERTEX_INPUT) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::VERTEX_SHADER) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::GEOMETRY_SHADER) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::FRAGMENT_SHADER) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::LATE_FRAGMENT_TESTS) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    if stages.subset(vk::PipelineStageFlags::COMPUTE_SHADER) {
        capability |= vk::QueueFlags::COMPUTE;
    }
    if stages.subset(vk::PipelineStageFlags::TRANSFER) {
        capability |= vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER;
    }
    if stages.subset(vk::PipelineStageFlags::ALL_GRAPHICS) {
        capability |= vk::QueueFlags::GRAPHICS;
    }
    capability
}
