
use std::{any::Any, ops::Range};
use device::Device;
use error::MappingError;
use mapping::MappedRange;
use memory::Properties;

/// Block that owns a `Range` of the `Memory`.
/// Implementor must ensure that there can't be any other blocks
/// with overlapping range (either through type system or safety notes for unsafe functions).
/// Provides access to safe memory range mapping.
pub trait Block {
    /// Memory type.
    type Memory: Any;

    /// Get memory properties of the block.
    fn properties(&self) -> Properties;

    /// Get raw memory object.
    fn memory(&self) -> &Self::Memory;

    /// Get memory range owned by this block.
    fn range(&self) -> Range<u64>;

    /// Get mapping for the buffer range.
    /// Memory writes to the region performed by device become available for the host.
    fn map<'a, D>(&'a mut self, device: &D, range: Range<u64>) -> Result<MappedRange<'a, Self::Memory>, MappingError>
    where
        D: Device<Memory = Self::Memory>,
    ;

    /// Release memory mapping. Must be called after successful `map` call.
    /// Memory writes to the region performed by host become available for the device.
    /// Specified region must be sub-region of the mapped region.
    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<Memory = Self::Memory>,
    ;
}
