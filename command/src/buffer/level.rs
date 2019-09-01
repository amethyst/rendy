/// Command buffers of this level can be submitted to the command queues.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrimaryLevel;

/// Command buffers of this level can be executed as part of the primary buffers.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecondaryLevel;

/// Type-level buffer level flag.
/// It defines whether buffer can be submitted to the command queues
/// or executed as part of the primary buffers.
pub trait Level: Copy + Default + std::fmt::Debug + 'static {
    /// Get raw level value for command buffer allocation.
    fn raw_level(&self) -> gfx_hal::command::Level;
}

impl Level for PrimaryLevel {
    fn raw_level(&self) -> gfx_hal::command::Level {
        gfx_hal::command::Level::Primary
    }
}

impl Level for SecondaryLevel {
    fn raw_level(&self) -> gfx_hal::command::Level {
        gfx_hal::command::Level::Secondary
    }
}
