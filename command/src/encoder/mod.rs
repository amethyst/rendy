//! Encoder module docs.
//!

use crate::buffer::{CommandBuffer, RecordingState};

/// Encoder allow command recording in safe-ish abstract manner.
pub trait Encoder<B: gfx_hal::Backend, C = gfx_hal::QueueType> {
    /// Get raw command buffer.
    ///
    /// # Safety
    ///
    /// Safety of commands recording through raw buffer is covered by corresponding functions.
    /// Handle must not be used outside of `Encoder` scope.
    /// Encoder implicitly finishes buffer recording.
    unsafe fn raw(&mut self) -> &mut B::CommandBuffer;
}

impl<B, C, U, L, R> Encoder<B, C> for CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: gfx_hal::Backend,
{
    unsafe fn raw(&mut self) -> &mut B::CommandBuffer {
        CommandBuffer::raw(self)
    }
}
