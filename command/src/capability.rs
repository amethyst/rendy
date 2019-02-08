//! Capability module docs.

pub use gfx_hal::QueueType;

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
pub trait Capability: Copy + std::fmt::Debug + 'static {
    /// Try to create capability instance from queue_type.
    /// Instance will be created if all required queue_type set.
    fn from_queue_type(queue_type: QueueType) -> Option<Self>;

    /// Convert into `QueueType`
    fn into_queue_type(self) -> QueueType;
}

impl Capability for QueueType {
    fn from_queue_type(queue_type: QueueType) -> Option<Self> {
        Some(queue_type)
    }

    fn into_queue_type(self) -> QueueType {
        self
    }
}

impl Capability for Transfer {
    fn from_queue_type(_queue_type: QueueType) -> Option<Self> {
        Some(Transfer)
    }

    fn into_queue_type(self) -> QueueType {
        QueueType::Transfer
    }
}

impl Capability for Execute {
    fn from_queue_type(queue_type: QueueType) -> Option<Self> {
        match queue_type {
            QueueType::Transfer => None,
            _ => Some(Execute),
        }
    }

    fn into_queue_type(self) -> QueueType {
        QueueType::General
    }
}

impl Capability for Compute {
    fn from_queue_type(queue_type: QueueType) -> Option<Self> {
        match queue_type {
            QueueType::Compute | QueueType::General => Some(Compute),
            _ => None
        }
    }

    fn into_queue_type(self) -> QueueType {
        QueueType::Compute
    }
}

impl Capability for Graphics {
    fn from_queue_type(queue_type: QueueType) -> Option<Self> {
        match queue_type {
            QueueType::Graphics | QueueType::General => Some(Graphics),
            _ => None
        }
    }

    fn into_queue_type(self) -> QueueType {
        QueueType::Graphics
    }
}

impl Capability for General {
    fn from_queue_type(queue_type: QueueType) -> Option<Self> {
        match queue_type {
            QueueType::General => Some(General),
            _ => None
        }
    }

    fn into_queue_type(self) -> QueueType {
        QueueType::General
    }
}

/// Check if capability supported.
pub trait Supports<C>: Capability {
    /// Check runtime capability.
    fn supports(&self) -> Option<C>;

    /// Assert capability.
    fn assert(&self) {
        assert!(self.supports().is_some());
    }
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

impl<C> Supports<C> for gfx_hal::QueueType
where
    C: Capability,
{
    fn supports(&self) -> Option<C> {
        C::from_queue_type(*self)
    }
}
