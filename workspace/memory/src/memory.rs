
use hal;

/// Memory object.
#[derive(Copy, Clone, Debug)]
pub struct Memory<T> {
    pub(crate) properties: hal::memory::Properties,
    pub(crate) size: u64,
    pub(crate) raw: T,
}

impl<T> Memory<T> {
    /// Get memory properties.
    pub fn properties(&self) -> hal::memory::Properties {
        self.properties
    }

    /// Get memory size.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get raw memory.
    pub fn raw(&self) -> &T {
        &self.raw
    }

    /// Get raw memory.
    pub fn raw_mut(&mut self) -> &mut T {
        &mut self.raw
    }

    /// Create memory from raw object.
    pub unsafe fn from_raw(properties: hal::memory::Properties, size: u64, raw: T) -> Self {
        Memory {
            properties,
            raw,
            size,
        }
    }
}
