//! Encoder module docs.
//!

mod clear;

use ash::vk::{CommandBuffer, QueueFlags};
use crate::buffer::{Buffer, RecordingState};

pub use self::clear::*;

/// Encoder allow command recording in safe-ish abstract manner.
pub trait Encoder<C = QueueFlags> {
    /// Get raw command buffer.
    ///
    /// # Safety
    ///
    /// Safety of commands recording through raw buffer is covered by corresponding functions.
    /// Handle must not be used outside of `Encoder` scope.
    /// Encoder implicitly finishes buffer recording.
    unsafe fn raw(&mut self) -> CommandBuffer;
}

impl<C, U, L, R> Encoder<C> for Buffer<C, RecordingState<U>, L, R> {
    unsafe fn raw(&mut self) -> CommandBuffer {
        Buffer::raw(self)
    }
}
