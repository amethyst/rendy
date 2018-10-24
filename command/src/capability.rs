//! Capability module docs.

use ash::vk::{PipelineStageFlags, QueueFlags};


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
    fn from_flags(flags: QueueFlags) -> Option<Self>;

    /// Convert into `QueueFlags`
    fn into_flags(self) -> QueueFlags;
}

impl Capability for QueueFlags {
    fn from_flags(flags: QueueFlags) -> Option<Self> {
        Some(flags)
    }

    fn into_flags(self) -> QueueFlags {
        self
    }
}

impl Capability for Transfer {
    fn from_flags(flags: QueueFlags) -> Option<Self> {
        if flags.subset(QueueFlags::TRANSFER) {
            Some(Transfer)
        } else {
            None
        }
    }

    fn into_flags(self) -> QueueFlags {
        QueueFlags::TRANSFER
    }
}

impl Capability for Execute {
    fn from_flags(flags: QueueFlags) -> Option<Self> {
        if flags.intersects(QueueFlags::COMPUTE | QueueFlags::GRAPHICS) {
            Some(Execute)
        } else {
            None
        }
    }

    fn into_flags(self) -> QueueFlags {
        QueueFlags::COMPUTE | QueueFlags::GRAPHICS
    }
}

impl Capability for Compute {
    fn from_flags(flags: QueueFlags) -> Option<Self> {
        if flags.subset(QueueFlags::COMPUTE) {
            Some(Compute)
        } else {
            None
        }
    }

    fn into_flags(self) -> QueueFlags {
        QueueFlags::COMPUTE
    }
}

impl Capability for Graphics {
    fn from_flags(flags: QueueFlags) -> Option<Self> {
        if flags.subset(QueueFlags::GRAPHICS) {
            Some(Graphics)
        } else {
            None
        }
    }

    fn into_flags(self) -> QueueFlags {
        QueueFlags::GRAPHICS
    }
}

impl Capability for General {
    fn from_flags(flags: QueueFlags) -> Option<Self> {
        if flags.subset(QueueFlags::GRAPHICS | QueueFlags::COMPUTE) {
            Some(General)
        } else {
            None
        }
    }

    fn into_flags(self) -> QueueFlags {
        QueueFlags::GRAPHICS | QueueFlags::COMPUTE
    }
}

/// Check if capability supported.
pub trait Supports<C> {
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

impl Supports<Transfer> for QueueFlags {
    fn supports(&self) -> Option<Transfer> {
        Transfer::from_flags(*self)
    }
}

impl Supports<Execute> for QueueFlags {
    fn supports(&self) -> Option<Execute> {
        Execute::from_flags(*self)
    }
}

impl Supports<Compute> for QueueFlags {
    fn supports(&self) -> Option<Compute> {
        Compute::from_flags(*self)
    }
}

impl Supports<Graphics> for QueueFlags {
    fn supports(&self) -> Option<Graphics> {
        Graphics::from_flags(*self)
    }
}

/// Get capabilities required by pipeline stages.
pub fn required_queue_capability(stages: PipelineStageFlags) -> QueueFlags {
    let mut capability = QueueFlags::empty();
    if stages.subset(PipelineStageFlags::DRAW_INDIRECT) {
        capability |= QueueFlags::GRAPHICS | QueueFlags::COMPUTE;
    }
    if stages.subset(PipelineStageFlags::VERTEX_INPUT) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::VERTEX_SHADER) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::TESSELLATION_CONTROL_SHADER) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::TESSELLATION_EVALUATION_SHADER) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::GEOMETRY_SHADER) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::FRAGMENT_SHADER) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::EARLY_FRAGMENT_TESTS) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::LATE_FRAGMENT_TESTS) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT) {
        capability |= QueueFlags::GRAPHICS;
    }
    if stages.subset(PipelineStageFlags::COMPUTE_SHADER) {
        capability |= QueueFlags::COMPUTE;
    }
    if stages.subset(PipelineStageFlags::TRANSFER) {
        capability |=
            QueueFlags::GRAPHICS | QueueFlags::COMPUTE | QueueFlags::TRANSFER;
    }
    if stages.subset(PipelineStageFlags::ALL_GRAPHICS) {
        capability |= QueueFlags::GRAPHICS;
    }
    capability
}
