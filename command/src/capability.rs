//! Capability module docs.

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
    /// Try to create capability instance from queue_type.
    /// Instance will be created if all required queue_type set.
    fn from_flags(queue_type: gfx_hal::QueueType) -> Option<Self>;

    /// Convert into `gfx_hal::QueueType`
    fn into_flags(self) -> gfx_hal::QueueType;
}

impl Capability for gfx_hal::QueueType {
    fn from_flags(queue_type: gfx_hal::QueueType) -> Option<Self> {
        Some(queue_type)
    }

    fn into_flags(self) -> gfx_hal::QueueType {
        self
    }
}

impl Capability for Transfer {
    fn from_flags(queue_type: gfx_hal::QueueType) -> Option<Self> {
        Some(Transfer)
    }

    fn into_flags(self) -> gfx_hal::QueueType {
        gfx_hal::QueueType::Transfer
    }
}

impl Capability for Execute {
    fn from_flags(queue_type: gfx_hal::QueueType) -> Option<Self> {
        match queue_type {
            _ => Some(Execute),
            gfx_hal::QueueType::Transfer => None,
        }
    }

    fn into_flags(self) -> gfx_hal::QueueType {
        gfx_hal::QueueType::General
    }
}

impl Capability for Compute {
    fn from_flags(queue_type: gfx_hal::QueueType) -> Option<Self> {
        match queue_type {
            gfx_hal::QueueType::Compute | gfx_hal::QueueType::General => Some(Compute),
            _ => None
        }
    }

    fn into_flags(self) -> gfx_hal::QueueType {
        gfx_hal::QueueType::Compute
    }
}

impl Capability for Graphics {
    fn from_flags(queue_type: gfx_hal::QueueType) -> Option<Self> {
        match queue_type {
            gfx_hal::QueueType::Graphics | gfx_hal::QueueType::General => Some(Graphics),
            _ => None
        }
    }

    fn into_flags(self) -> gfx_hal::QueueType {
        gfx_hal::QueueType::Graphics
    }
}

impl Capability for General {
    fn from_flags(queue_type: gfx_hal::QueueType) -> Option<Self> {
        match queue_type {
            gfx_hal::QueueType::General => Some(General),
            _ => None
        }
    }

    fn into_flags(self) -> gfx_hal::QueueType {
        gfx_hal::QueueType::General
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

impl Supports<Transfer> for gfx_hal::QueueType {
    fn supports(&self) -> Option<Transfer> {
        Transfer::from_flags(*self)
    }
}

impl Supports<Execute> for gfx_hal::QueueType {
    fn supports(&self) -> Option<Execute> {
        Execute::from_flags(*self)
    }
}

impl Supports<Compute> for gfx_hal::QueueType {
    fn supports(&self) -> Option<Compute> {
        Compute::from_flags(*self)
    }
}

impl Supports<Graphics> for gfx_hal::QueueType {
    fn supports(&self) -> Option<Graphics> {
        Graphics::from_flags(*self)
    }
}
