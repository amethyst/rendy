
use std::{ops::Range};
use memory::{Device, MappingError, Properties};

/// Block<T> owns a `Range` of the `Memory<T>`.
/// It also provides a way to get mapping for the sub-range.
pub trait Block<T> {
    /// Get memory properties of the block.
    fn properties(&self) -> Properties;

    /// Get raw memory object.
    fn memory(&self) -> &T;

    /// Get memory range owned by this block.
    fn range(&self) -> Range<u64>;

    /// Get mapping for the buffer range.
    /// Memory writes to the region performed by device become available for the host.
    fn map<D>(&mut self, device: &D, range: Range<u64>) -> Result<&mut [u8], MappingError>
    where
        D: Device<T>,
    ;

    /// Release memory mapping. Must be called after successful `map` call.
    /// Memory writes to the region performed by host become available for the device.
    /// Specified region must be sub-region of the mapped region.
    fn unmap<D>(&mut self, device: &D, range: Range<u64>)
    where
        D: Device<T>,
    ;
}
