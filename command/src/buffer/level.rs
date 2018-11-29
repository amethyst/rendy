
/// Command buffers of this level can be submitted to the command queues.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrimaryLevel;

/// Command buffers of this level can be executed as part of the primary buffers.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecondaryLevel;

/// Command buffer level.
pub trait Level: Copy {
    /// Get raw level value.
    fn level(&self) -> gfx_hal::command::RawLevel;
}

impl Level for PrimaryLevel {
    fn level(&self) -> gfx_hal::command::RawLevel {
        gfx_hal::command::RawLevel::Primary
    }
}

impl Level for SecondaryLevel {
    fn level(&self) -> gfx_hal::command::RawLevel {
        gfx_hal::command::RawLevel::Secondary
    }
}

impl Level for gfx_hal::command::RawLevel {
    fn level(&self) -> gfx_hal::command::RawLevel {
        *self
    }
}
