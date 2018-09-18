use relevant::Relevant;

bitflags! {
    /// Memory property flags.
    /// Bitmask specifying properties for a memory type.
    /// See Vulkan docs for detailed info:
    /// <https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkMemoryPropertyFlagBits.html>
    #[repr(transparent)]
    pub struct Properties: u32 {
        /// Specifies that memory allocated with this type is the most efficient for device access.
        /// This property will be set if and only if the memory type belongs to a heap with the DEVICE_LOCAL bit set.
        const DEVICE_LOCAL = 0x00000001;

        /// Specifies that memory allocated with this type can be mapped for host access using `Device::map`.
        const HOST_VISIBLE = 0x00000002;

        /// Specifies that the host cache management commands
        /// `Device::flush` and `Device::invalidate` are not needed
        /// to flush host writes to the device or make device writes visible to the host, respectively.
        const HOST_COHERENT = 0x00000004;

        /// Specifies that memory allocated with this type is cached on the host.
        /// Host memory accesses to uncached memory are slower than to cached memory,
        /// however uncached memory is always host coherent.
        const HOST_CACHED = 0x00000008;

        /// Specifies that the memory type only allows device access to the memory.
        /// Memory types must not have both `LAZILY_ALLOCATED` and `HOST_VISIBLE` set.
        /// Additionally, the object’s backing memory may be provided by the implementation lazily as specified in [Lazily Allocated Memory](https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#memory-device-lazy_allocation).
        const LAZILY_ALLOCATED = 0x00000010;

        /// Specifies that the memory type only allows device access to the memory,
        /// and allows protected queue operations to access the memory.
        /// Memory types must not have `PROTECTED` bit set and any of `HOST_VISIBLE` bit set, or `HOST_COHERENT` bit set, or `HOST_CACHED` bit set.
        const PROTECTED = 0x00000020;
    }
}

impl Properties {
    /// Check if memory with this properties local for device.
    /// Implies fast access by the device.
    pub fn device_local(self) -> bool {
        self.contains(Self::DEVICE_LOCAL)
    }

    /// Check if memory with this properties visible to host.
    /// Can be mapped to the host memory.
    pub fn host_visible(self) -> bool {
        self.contains(Self::HOST_VISIBLE)
    }

    /// Check if host access to the mapped range of the memory with this properties is coherent.
    /// Mapped range of the non-coherent memory must be:
    /// * invalidated to make device writes available to the host
    /// * flushed to make host writes available to the device
    pub fn host_coherent(self) -> bool {
        self.contains(Self::HOST_COHERENT)
    }

    /// Check if host access to the mapped region of the memory with this properties is done through cache.
    /// Cached read can be faster for the host to perform.
    /// Prefer cached memory for 'device to host' data flow.
    pub fn host_cached(self) -> bool {
        self.contains(Self::HOST_CACHED)
    }

    /// Check if memory with this properties allow lazy allocation.
    /// Lazy memory could be used for transient attachments.
    pub fn lazily_allocated(self) -> bool {
        self.contains(Self::LAZILY_ALLOCATED)
    }

    /// Check if protected queue operations allowed to access memory with this properties.
    pub fn protected(self) -> bool {
        self.contains(Self::PROTECTED)
    }
}

/// Memory object wrapper.
/// Contains size and properties of the memory.
#[derive(Debug)]
pub struct Memory<T> {
    raw: T,
    size: u64,
    properties: Properties,
    relevant: Relevant,
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
        self.relevant.dispose();
        self.raw
    }

    /// Create memory from raw object.
    pub unsafe fn from_raw(raw: T, size: u64, properties: Properties) -> Self {
        Memory {
            properties,
            raw,
            size,
            relevant: Relevant,
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
