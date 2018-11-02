//! Encoder module docs.
//!

mod clear;

use ash::vk;
use crate::buffer::{CommandBuffer, RecordingState};

pub use self::clear::*;

/// Encoder allow command recording in safe-ish abstract manner.
pub trait Encoder<C = vk::QueueFlags> {
    /// Get raw command buffer.
    ///
    /// # Safety
    ///
    /// Safety of commands recording through raw buffer is covered by corresponding functions.
    /// Handle must not be used outside of `Encoder` scope.
    /// Encoder implicitly finishes buffer recording.
    unsafe fn raw(&mut self) -> vk::CommandBuffer;
}

impl<C, U, L, R> Encoder<C> for CommandBuffer<C, RecordingState<U>, L, R> {
    unsafe fn raw(&mut self) -> vk::CommandBuffer {
        CommandBuffer::raw(self)
    }
}
