// use std::fmt;
use ash::vk;
use relevant::Relevant;

/// Memory object wrapper.
/// Contains size and properties of the memory.
#[derive(Debug)]
pub struct Memory {
    raw: vk::DeviceMemory,
    size: u64,
    properties: vk::MemoryPropertyFlags,
    relevant: Relevant,
}

impl Memory {
    /// Get memory properties.
    pub fn properties(&self) -> vk::MemoryPropertyFlags {
        self.properties
    }

    /// Get memory size.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get raw memory.
    pub fn raw(&self) -> vk::DeviceMemory {
        self.raw
    }

    /// Create memory from raw object.
    ///
    /// # Safety
    ///
    /// TODO:
    pub unsafe fn from_raw(
        raw: vk::DeviceMemory,
        size: u64,
        properties: vk::MemoryPropertyFlags,
    ) -> Self {
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
        self.properties
            .subset(vk::MemoryPropertyFlags::HOST_VISIBLE)
    }

    /// Check if this memory is host-coherent and doesn't require invalidating or flushing.
    /// `memory.host_coherent()` is equivalent to `memory.properties().subset(Properties::HOST_COHERENT)`
    pub fn host_coherent(&self) -> bool {
        self.properties
            .subset(vk::MemoryPropertyFlags::HOST_COHERENT)
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
