use std::ops::Range;

use ash::{version::DeviceV1_0, vk::{DeviceMemory, MemoryPropertyFlags}};

use error::MappingError;
use mapping::MappedRange;


/// Block that owns a `Range` of the `Memory`.
/// Implementor must ensure that there can't be any other blocks
/// with overlapping range (either through type system or safety notes for unsafe functions).
/// Provides access to safe memory range mapping.
pub trait Block {
    /// Get memory properties of the block.
    fn properties(&self) -> MemoryPropertyFlags;

    /// Get raw memory object.
    fn memory(&self) -> DeviceMemory;

    /// Get memory range owned by this block.
    fn range(&self) -> Range<u64>;

    /// Get mapping for the buffer range.
    /// Memory writes to the region performed by device become available for the host.
    fn map<'a>(
        &'a mut self,
        device: &impl DeviceV1_0,
        range: Range<u64>,
    ) -> Result<MappedRange<'a>, MappingError>;

    /// Release memory mapping. Must be called after successful `map` call.
    /// No-op if block is not mapped.
    fn unmap(&mut self, device: &impl DeviceV1_0);
}
