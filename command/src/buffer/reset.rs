
use super::{
    state::*,
    usage::*,
};

/// This flag specify that buffer can be reset individually.
#[derive(Clone, Copy, Debug, Default)]
pub struct IndividualReset;

/// This flag specify that buffer cannot be reset individually.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoIndividualReset;

/// Specify flags required for command pool creation to allow individual buffer reset.
pub trait Reset: Copy + Default + std::fmt::Debug + 'static {
    /// Get flags for reset parameter.
    fn flags(&self) -> gfx_hal::pool::CommandPoolCreateFlags;
}

impl Reset for IndividualReset {
    fn flags(&self) -> gfx_hal::pool::CommandPoolCreateFlags {
        gfx_hal::pool::CommandPoolCreateFlags::RESET_INDIVIDUAL
    }
}

impl Reset for NoIndividualReset {
    fn flags(&self) -> gfx_hal::pool::CommandPoolCreateFlags {
        gfx_hal::pool::CommandPoolCreateFlags::empty()
    }
}

/// States in which command buffer can de reset.
pub trait Resettable: Copy + Default + std::fmt::Debug + 'static {}
impl Resettable for InitialState {}
impl<U, P> Resettable for RecordingState<U, P> where U: Usage, P: Copy + Default + std::fmt::Debug + 'static {}
impl<U, P> Resettable for ExecutableState<U, P> where U: Usage, P: Copy + Default + std::fmt::Debug + 'static {}
impl Resettable for InvalidState {}
