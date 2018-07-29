
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

#[cfg(feature = "gfx-hal")]
impl From<::hal::memory::Properties> for Properties {
    fn from(value: ::hal::memory::Properties) -> Self {
        let mut result = Properties::empty();
        if value.contains(::hal::memory::Properties::DEVICE_LOCAL) {
            result |= Properties::DEVICE_LOCAL;
        }
        if value.contains(::hal::memory::Properties::COHERENT) {
            result |= Properties::HOST_COHERENT;
        }
        if value.contains(::hal::memory::Properties::CPU_CACHED) {
            result |= Properties::HOST_CACHED;
        }
        if value.contains(::hal::memory::Properties::CPU_VISIBLE) {
            result |= Properties::HOST_VISIBLE;
        }
        if value.contains(::hal::memory::Properties::LAZILY_ALLOCATED) {
            result |= Properties::LAZILY_ALLOCATED;
        }
        result
    }
}

#[cfg(feature = "gfx-hal")]
impl Into<::hal::memory::Properties> for Properties {
    fn into(self) -> ::hal::memory::Properties {
        let mut result = ::hal::memory::Properties::empty();
        if self.contains(Properties::DEVICE_LOCAL) {
            result |= ::hal::memory::Properties::DEVICE_LOCAL;
        }
        if self.contains(Properties::HOST_COHERENT) {
            result |= ::hal::memory::Properties::COHERENT;
        }
        if self.contains(Properties::HOST_CACHED) {
            result |= ::hal::memory::Properties::CPU_CACHED;
        }
        if self.contains(Properties::HOST_VISIBLE) {
            result |= ::hal::memory::Properties::CPU_VISIBLE;
        }
        if self.contains(Properties::LAZILY_ALLOCATED) {
            result |= ::hal::memory::Properties::LAZILY_ALLOCATED;
        }
        result
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

/// Trait for memory allocation and mapping.
pub trait Device<T> {
    /// Allocate memory object.
    /// 
    /// # Parameters
    /// `size`  - size of the memory object to allocate.
    /// `index` - memory type index.
    unsafe fn allocate(&self, index: u32, size: u64) -> Result<T, MemoryError>;

    /// Free memory object.
    unsafe fn free(&self, memory: T);

    /// Map memory range.
    /// Only one range for the given memory object can be mapped.
    unsafe fn map(&self, memory: &T, range: Range<u64>) -> Result<NonNull<u8>, MappingError>;

    /// Unmap memory.
    unsafe fn unmap(&self, memory: &T);

    /// Invalidate mapped regions guaranteeing that device writes to the memory,
    /// which have been made visible to the host-write and host-read access types, are made visible to the host
    unsafe fn invalidate<'a>(&self, regions: impl IntoIterator<Item = (&'a T, Range<u64>)>)
    where
        T: 'a,
    ;

    /// Flush mapped regions guaranteeing that host writes to the memory can be made available to device access
    unsafe fn flush<'a>(&self, regions: impl IntoIterator<Item = (&'a T, Range<u64>)>)
    where
        T: 'a,
    ;
}

#[cfg(feature = "gfx-hal")]
impl<D, B> Device<B::Memory> for (D, ::std::marker::PhantomData<B>)
where
    B: ::hal::Backend,
    D: ::std::borrow::Borrow<B::Device>,
{
    unsafe fn allocate(&self, index: u32, size: u64) -> Result<B::Memory, MemoryError> {
        use hal::Device as HalDevice;
        match self.0.borrow().allocate_memory(::hal::MemoryTypeId(index as usize), size) {
            Ok(memory) => Ok(memory),
            Err(::hal::device::OutOfMemory) => Err(MemoryError::OutOfDeviceMemory),
        }
    }

    unsafe fn free(&self, memory: B::Memory) {
        use hal::Device as HalDevice;
        self.0.borrow().free_memory(memory)
    }

    unsafe fn map(&self, memory: &B::Memory, range: Range<u64>) -> Result<NonNull<u8>, MappingError> {
        use hal::Device as HalDevice;
        match self.0.borrow().map_memory(memory, range) {
            Ok(ptr) => {
                debug_assert!(!ptr.is_null());
                Ok(NonNull::new_unchecked(ptr))
            }
            Err(::hal::mapping::Error::InvalidAccess) => Err(MappingError::HostInvisible),
            Err(::hal::mapping::Error::OutOfBounds) => Err(MappingError::OutOfBounds),
            Err(::hal::mapping::Error::OutOfMemory) => Err(MappingError::OutOfHostMemory),
        }
    }

    unsafe fn unmap(&self, memory: &B::Memory) {
        use hal::Device as HalDevice;
        self.0.borrow().unmap_memory(memory)
    }

    unsafe fn invalidate<'a>(&self, regions: impl IntoIterator<Item = (&'a B::Memory, Range<u64>)>) {
        use hal::Device as HalDevice;
        self.0.borrow().invalidate_mapped_memory_ranges(regions)
    }

    unsafe fn flush<'a>(&self, regions: impl IntoIterator<Item = (&'a B::Memory, Range<u64>)>) {
        use hal::Device as HalDevice;
        self.0.borrow().flush_mapped_memory_ranges(regions)
    }
}

/// Typical memory error - out of available memory.
#[derive(Clone, Debug, Fail)]
pub enum MemoryError {
    /// Host memory exhausted.
    #[fail(display = "Out of host memory")]
    OutOfHostMemory,

    /// Device memory exhausted.
    #[fail(display = "Out of device memory")]
    OutOfDeviceMemory,
}

#[derive(Clone, Debug, Fail)]
pub enum MappingError {
    /// Attempt to map memory without host-visible property.
    #[fail(display = "Memory is not HOST_VISIBLE and can't be mapped")]
    HostInvisible,

    /// Host memory exhausted.
    #[fail(display = "Out of host memory")]
    OutOfHostMemory,

    /// Attempt to bound memory out of memory bounds.
    #[fail(display = "Mapping range is out of bound")]
    OutOfBounds,
}


