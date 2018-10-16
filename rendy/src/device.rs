use rendy_command::Device as CommandDevice;
use rendy_memory::Device as MemoryDevice;
use rendy_resource::Device as ResourceDevice;

pub trait Device: MemoryDevice + ResourceDevice + CommandDevice {}
