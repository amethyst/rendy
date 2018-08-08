
use std::{ops::Range, ptr::NonNull};

/// Memory property flags.
bitflags! {
    pub struct Properties: u32 {
        /// Specifies that memory allocated with this type is the most efficient for device access.
        const DEVICE_LOCAL = 0x00000001;

        /// Specifies that memory allocated with this type can be mapped for host access.
        const HOST_VISIBLE = 0x00000002;

        /// Specifies that the host cache management commands `Device::flush` and `Device::invalidate`
        /// are not needed to flush host writes to the device or make device writes visible to the host, respectively.
        const HOST_COHERENT = 0x00000004;

        /// Specifies that memory allocated with this type is cached on the host.
        /// Host memory accesses to uncached memory are slower than to cached memory,
        /// however uncached memory is always host coherent.
        const HOST_CACHED = 0x00000008;

        /// Specifies that the memory type only allows device access to the memory.
        /// Memory types must not have both `LAZILY_ALLOCATED` and `HOST_VISIBLE` set.
        /// Additionally, the object’s backing memory may be provided by the implementation lazily
        /// as specified in (https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#memory-device-lazy_allocation)[Lazily Allocated Memory].
        const LAZILY_ALLOCATED = 0x00000010;

        /// Specifies that the memory type only allows device access to the memory,
        /// and allows protected queue operations to access the memory.
        /// Memory types must not have `PROTECTED` set and any of `HOST_VISIBLE` set,
        /// or `HOST_COHERENT` set, or `HOST_CACHED` set.
        const PROTECTED = 0x00000020;
    }
}

impl Properties {
    pub fn device_local(self) -> bool {
        self.contains(Self::DEVICE_LOCAL)
    }
    pub fn host_visible(self) -> bool {
        self.contains(Self::HOST_VISIBLE)
    }
    pub fn host_coherent(self) -> bool {
        self.contains(Self::HOST_COHERENT)
    }
    pub fn host_cached(self) -> bool {
        self.contains(Self::HOST_CACHED)
    }
    pub fn lazily_allocated(self) -> bool {
        self.contains(Self::LAZILY_ALLOCATED)
    }
    pub fn protected(self) -> bool {
        self.contains(Self::PROTECTED)
    }
}

/// Memory object wrapper.
/// Contains size and properties of the memory.
#[derive(Copy, Clone, Debug)]
pub struct Memory<T> {
    pub(crate) raw: T,
    pub(crate) size: u64,
    pub(crate) properties: Properties,
}

impl<T> Memory<T> {
    /// Get memory properties.
    pub fn properties(&self) -> Properties {
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

    /// Convert into raw
    pub fn into_raw(self) -> T {
        self.raw
    }

    /// Create memory from raw object.
    pub unsafe fn from_raw(raw: T, size: u64, properties: Properties) -> Self {
        Memory {
            properties,
            raw,
            size,
        }
    }

    /// Check if this memory is host-visible and can be mapped.
    /// `memory.host_visible()` is equivalent to `memory.properties().contains(Properties::HOST_VISIBLE)`
    pub fn host_visible(&self) -> bool {
        self.properties.contains(Properties::HOST_VISIBLE)
    }

    /// Check if this memory is host-coherent and doesn't require invalidating or flushing.
    /// `memory.host_coherent()` is equivalent to `memory.properties().contains(Properties::HOST_COHERENT)`
    pub fn host_coherent(&self) -> bool {
        self.properties.contains(Properties::HOST_COHERENT)
    }
}
