//! Encoder module docs.
//!

mod clear;

use ash::vk::{CommandBuffer, QueueFlags};

pub use self::clear::*;

/// Encoder allow command recording in safe-ish abstract manner.
pub trait Encoder<C = QueueFlags> {
    /// Get inner raw command buffer.
    ///
    /// # Safety
    ///
    /// Safety of commands recording through raw buffer is covered by corresponding functions.
    /// Handle must not be used outside of `Encoder` scope.
    /// Encoder implicitly finishes buffer recording.
    unsafe fn raw(&mut self) -> CommandBuffer;
}
