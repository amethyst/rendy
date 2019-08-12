use std::ops::Range;

use crate::mapping::MappedRange;

/// Block that owns a `Range` of the `Memory`.
/// Implementor must ensure that there can't be any other blocks
/// with overlapping range (either through type system or safety notes for unsafe functions).
/// Provides access to safe memory range mapping.
pub trait Block<B: rendy_core::hal::Backend> {
    /// Get memory properties of the block.
    fn properties(&self) -> rendy_core::hal::memory::Properties;

    /// Get raw memory object.
    fn memory(&self) -> &B::Memory;

    /// Get memory range owned by this block.
    fn range(&self) -> Range<u64>;

    /// Get size of the block.
    fn size(&self) -> u64 {
        let range = self.range();
        range.end - range.start
    }

    /// Get mapping for the buffer range.
    /// Memory writes to the region performed by device become available for the host.
    fn map<'a>(
        &'a mut self,
        device: &B::Device,
        range: Range<u64>,
    ) -> Result<MappedRange<'a, B>, rendy_core::hal::mapping::Error>;

    /// Release memory mapping. Must be called after successful `map` call.
    /// No-op if block is not mapped.
    fn unmap(&mut self, device: &B::Device);
}
