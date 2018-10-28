use ash;
use usage::MemoryUsageValue;

/// Typical memory error - out of available memory.
#[derive(Clone, Copy, Debug, Fail)]
pub enum OutOfMemoryError {
    /// Host memory exhausted.
    #[fail(display = "Out of host memory")]
    OutOfHostMemory,

    /// Device memory exhausted.
    #[fail(display = "Out of device memory")]
    OutOfDeviceMemory,

    /// Allocator detects that all suitable heaps are exhausted.
    #[fail(display = "All suitable heaps are exhausted")]
    HeapsExhausted,
}

/// Possible cause of mapping failure.
#[derive(Clone, Copy, Debug, Fail)]
pub enum MappingError {
    /// Attempt to map memory without host-visible property.
    #[fail(display = "Memory is not HOST_VISIBLE and can't be mapped")]
    HostInvisible,

    /// Attempt to map memory out of bounds.
    #[fail(display = "Mapping range is out of bound")]
    OutOfBounds,

    /// Attempt to map memory that can't be safely mapped due to allocator limits.
    /// User still may perform mapping manually.
    /// Note that manual mapping is unsafe.
    #[fail(display = "Memory can't be mapped safely due to allocator limits")]
    MappingUnsafe,

    /// Unable to allocate an appropriately sized contiguous virtual address range
    #[fail(display = "Virtual memory allocation failed")]
    MappingFailed,

    /// Out of either host or device memory.
    #[fail(display = "{}", _0)]
    OutOfMemoryError(OutOfMemoryError),

    /// Attempt to interpret mapped range with wrong alignment.
    #[fail(
        display = "Offset {} doesn't satisfy alignment requirements {}",
        offset,
        align
    )]
    Unaligned {
        /// Alignment requirements.
        align: usize,

        /// Offset that doesn't satisfy alignment.
        offset: usize,
    },
}

impl From<OutOfMemoryError> for MappingError {
    fn from(error: OutOfMemoryError) -> Self {
        MappingError::OutOfMemoryError(error)
    }
}

/// Possible cause of allocation failure.
#[derive(Clone, Copy, Debug, Fail)]
pub enum AllocationError {
    /// Out of either host or device memory.
    #[fail(display = "{}", _0)]
    OutOfMemoryError(OutOfMemoryError),

    /// Vulkan implementation doesn't allow to create too many objects.
    #[fail(display = "Can't allocate more memory objects")]
    TooManyObjects,

    /// No memory types for specified mask and usage were found.
    #[fail(
        display = "No suitable memory types for mask: ({}) and usage: ({:?})",
        _0,
        _1
    )]
    NoSuitableMemory(u32, MemoryUsageValue),
}

impl From<OutOfMemoryError> for AllocationError {
    fn from(error: OutOfMemoryError) -> Self {
        AllocationError::OutOfMemoryError(error)
    }
}

/// Generic memory error.
#[derive(Clone, Copy, Debug, Fail)]
pub enum MemoryError {
    /// Out of either host or device memory.
    #[fail(display = "{}", _0)]
    OutOfMemoryError(OutOfMemoryError),

    /// Error occurred during mapping operation.
    #[fail(display = "{}", _0)]
    MappingError(MappingError),

    /// Error occurred during allocation.
    #[fail(display = "{}", _0)]
    AllocationError(AllocationError),
}

impl From<OutOfMemoryError> for MemoryError {
    fn from(error: OutOfMemoryError) -> Self {
        MemoryError::OutOfMemoryError(error)
    }
}

impl From<AllocationError> for MemoryError {
    fn from(error: AllocationError) -> Self {
        MemoryError::AllocationError(error)
    }
}

impl From<MappingError> for MemoryError {
    fn from(error: MappingError) -> Self {
        MemoryError::MappingError(error)
    }
}

impl From<ash::vk::Result> for OutOfMemoryError {
    fn from(result: ash::vk::Result) -> OutOfMemoryError {
        match result {
            ash::vk::Result::SUCCESS => panic!("Unexpected success"),
            ash::vk::Result::ERROR_OUT_OF_HOST_MEMORY => OutOfMemoryError::OutOfHostMemory,
            ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => OutOfMemoryError::OutOfDeviceMemory,
            _ => panic!("unexpected error"),
        }
    }
}

impl From<ash::vk::Result> for MappingError {
    fn from(result: ash::vk::Result) -> MappingError {
        match result {
            ash::vk::Result::SUCCESS => panic!("Unexpected success"),
            ash::vk::Result::ERROR_OUT_OF_HOST_MEMORY => OutOfMemoryError::OutOfHostMemory.into(),
            ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => {
                OutOfMemoryError::OutOfDeviceMemory.into()
            }
            ash::vk::Result::ERROR_MEMORY_MAP_FAILED => MappingError::MappingFailed,
            _ => panic!("unexpected error"),
        }
    }
}

impl From<ash::vk::Result> for AllocationError {
    fn from(result: ash::vk::Result) -> AllocationError {
        match result {
            ash::vk::Result::SUCCESS => panic!("Unexpected success"),
            ash::vk::Result::ERROR_OUT_OF_HOST_MEMORY => OutOfMemoryError::OutOfHostMemory.into(),
            ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => {
                OutOfMemoryError::OutOfDeviceMemory.into()
            }
            _ => panic!("unexpected error"),
        }
    }
}

impl From<ash::vk::Result> for MemoryError {
    fn from(result: ash::vk::Result) -> MemoryError {
        match result {
            ash::vk::Result::SUCCESS => panic!("Unexpected success"),
            ash::vk::Result::ERROR_OUT_OF_HOST_MEMORY => OutOfMemoryError::OutOfHostMemory.into(),
            ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => {
                OutOfMemoryError::OutOfDeviceMemory.into()
            }
            ash::vk::Result::ERROR_MEMORY_MAP_FAILED => MappingError::MappingFailed.into(),
            _ => panic!("unexpected error"),
        }
    }
}
