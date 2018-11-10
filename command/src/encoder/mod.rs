//! Encoder module docs.
//!

use crate::capability::Supports;

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