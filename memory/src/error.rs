

/// Typical memory error - out of available memory.
#[derive(Clone, Debug, Fail)]
pub enum OutOfMemoryError {
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

    /// Attempt to bound memory out of memory bounds.
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

    #[fail(display = "{}", _0)]
    OutOfMemoryError(OutOfMemoryError),
}

impl From<OutOfMemoryError> for MappingError {
    fn from(error: OutOfMemoryError) -> Self {
        MappingError::OutOfMemoryError(error)
    }
}

#[derive(Clone, Debug, Fail)]
pub enum AllocationError {
    #[fail(display = "{}", _0)]
    OutOfMemoryError(OutOfMemoryError),

    #[fail(display = "Can't allocate more memory objects")]
    TooManyObjects,
}

impl From<OutOfMemoryError> for AllocationError {
    fn from(error: OutOfMemoryError) -> Self {
        AllocationError::OutOfMemoryError(error)
    }
}

#[derive(Clone, Debug, Fail)]
pub enum MemoryError {
    #[fail(display = "{}", _0)]
    OutOfMemoryError(OutOfMemoryError),

    #[fail(display = "{}", _0)]
    MappingError(MappingError),

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
