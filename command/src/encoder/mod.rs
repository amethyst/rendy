//! Encoder module docs.
//!

mod clear;

pub use self::clear::*;

use capability::CapabilityFlags;
use device::CommandBuffer;

/// Encoder allow command recording in safe-ish abstract manner.
pub trait Encoder<C = CapabilityFlags> {
    /// Get command buffer.
    type Buffer: CommandBuffer;

    /// Get inner raw command buffer.
    ///
    /// # Safety
    ///
    /// Safety of commands recording through raw buffer is covered by corresponding functions.
    /// Yet this method is unsafe because:
    /// * Moving out raw buffer is very unsafe and should be avoided.
    /// * Creating copies can be safe only if copies don't outlive encoder instance.
    unsafe fn buffer(&mut self) -> &mut Self::Buffer;
}
