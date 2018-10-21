use command::Device as CommandDevice;
use memory::Device as MemoryDevice;
use resource::Device as ResourceDevice;

/// Collective trait that represents the capabilites a device used in
/// `rendy` must have.
pub trait Device: MemoryDevice + ResourceDevice + CommandDevice {
    type Surface;
}
