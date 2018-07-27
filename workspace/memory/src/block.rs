
use std::{fmt::Debug, ops::Range};
use hal;

#[derive(Clone, Debug, Fail)]
pub enum MappingError {
    #[fail(display = "Memory is not HOST_VISIBLE and can't be mapped")]
    HostInvisible,

    #[fail(display = "Out of host memory")]
    OutOfHostMemory,

    #[fail(display = "Mapping range is out of bound")]
    OutOfBounds,
}

/// Memory block trait implemented for blocks allocated by allocators.
pub trait Block<T: Debug + Send + Sync + 'static> {
    /// Get memory properties of the block.
    fn properties(&self) -> hal::memory::Properties;

    /// Get memory object.
    fn memory(&self) -> &T;

    /// Get memory range associated with this block.
    fn range(&self) -> Range<u64>;

    /// Get mapping for the buffer range.
    /// Memory writes to the region performed by device become available for the host.
    fn map<B>(&mut self, device: &B::Device, range: Range<u64>) -> Result<&mut [u8], MappingError>
    where
        B: hal::Backend<Memory = T>,
    ;

    /// Release memory mapping. Must be called after successful `map` call.
    /// Memory writes to the region performed by host become available for the device.
    /// Specified region must be sub-region of the mapped region.
    fn unmap<B>(&mut self, device: &B::Device, range: Range<u64>)
    where
        B: hal::Backend<Memory = T>,
    ;
}
