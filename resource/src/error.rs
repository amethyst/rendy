use image;
use memory;

/// Image creation error.
#[derive(Clone, Copy, Debug, Fail)]
pub enum ImageCreationError {
    /// An unsupported format was attempted to be used.
    #[fail(display = "Unsupported format")]
    UnsupportedFormat(image::Format),

    /// Multi-sampled array textures or cubes are not supported.
    #[fail(display = "Unsupported kind")]
    Kind,

    /// Invalid samples for the device.
    #[fail(display = "Unsupported amount of samples")]
    Samples(image::SampleCountFlags),

    /// Unsupported size in one of the dimensions.
    #[fail(display = "Unsupported size")]
    UnsupportedSize(u32),

    /// The data size provided doesn't match the destination.
    #[fail(display = "Data size mismatch")]
    DataSizeMismatch,

    /// The usage requested isn't supported.
    #[fail(display = "Unsupported usage")]
    UnsupportedUsage(image::UsageFlags),

    /// The memory of the host or device is used up.
    #[fail(display = "Out of memory")]
    OutOfMemoryError(memory::OutOfMemoryError),
}

/// Resource binding error.
#[derive(Clone, Copy, Debug, Fail)]
pub enum BindError {
    /// Requested binding to memory that doesn't support the required operations.
    #[fail(display = "Binding to wrong memory")]
    WrongMemory,

    /// Requested binding to an invalid memory.
    #[fail(display = "Binding to out of bounds memory")]
    OutOfBounds,

    /// The memory of the host or device is used up.
    #[fail(display = "Out of memory")]
    OutOfMemoryError(memory::OutOfMemoryError),
}

/// Generic resource error.
#[derive(Clone, Copy, Debug, Fail)]
pub enum ResourceError {
    /// Image creation error.
    #[fail(display = "Image creation error")]
    ImageCreationError(ImageCreationError),

    /// Memory error.
    #[fail(display = "Memory error")]
    MemoryError(memory::MemoryError),

    /// Bind error.
    #[fail(display = "Bind error")]
    BindError(BindError),
}

impl From<ImageCreationError> for ResourceError {
    fn from(error: ImageCreationError) -> Self {
        ResourceError::ImageCreationError(error)
    }
}

impl From<memory::MemoryError> for ResourceError {
    fn from(error: memory::MemoryError) -> Self {
        ResourceError::MemoryError(error)
    }
}

impl From<BindError> for ResourceError {
    fn from(error: BindError) -> Self {
        ResourceError::BindError(error)
    }
}
