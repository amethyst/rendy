// use std::fmt;
use ash::vk::{DeviceMemory, MemoryPropertyFlags};
use relevant::Relevant;

/// Memory object wrapper.
/// Contains size and properties of the memory.
#[derive(Debug)]
pub struct Memory {
    raw: DeviceMemory,
    size: u64,
    properties: MemoryPropertyFlags,
    relevant: Relevant,
}

impl Memory {
    /// Get memory properties.
    pub fn properties(&self) -> MemoryPropertyFlags {
        self.properties
    }

    /// Get memory size.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get raw memory.
    pub fn raw(&self) -> DeviceMemory {
        self.raw
    }

    /// Create memory from raw object.
    pub unsafe fn from_raw(raw: DeviceMemory, size: u64, properties: MemoryPropertyFlags) -> Self {
        Memory {
            properties,
            raw,
            size,
            relevant: Relevant,
        }
    }

    /// Check if this memory is host-visible and can be mapped.
    /// `memory.host_visible()` is equivalent to `memory.properties().subset(Properties::HOST_VISIBLE)`
    pub fn host_visible(&self) -> bool {
        self.properties.subset(MemoryPropertyFlags::HOST_VISIBLE)
    }

    /// Check if this memory is host-coherent and doesn't require invalidating or flushing.
    /// `memory.host_coherent()` is equivalent to `memory.properties().subset(Properties::HOST_COHERENT)`
    pub fn host_coherent(&self) -> bool {
        self.properties.subset(MemoryPropertyFlags::HOST_COHERENT)
    }

    /// Dispose of memory object.
    pub(crate) fn dispose(self) {
        self.relevant.dispose();
    }
}

// pub(crate) fn memory_ptr_fmt(
//     memory: &*const Memory,
//     fmt: &mut fmt::Formatter<'_>,
// ) -> Result<(), fmt::Error> {
//     unsafe {
//         if fmt.alternate() {
//             write!(fmt, "*const {:#?}", **memory)
//         } else {
//             write!(fmt, "*const {:?}", **memory)
//         }
//     }
// }
