use buffer;
use image;
use memory;
use error;
use MemoryRequirements;

/// Trait for resource creation, memory allocation and mapping.
pub trait Device: memory::Device + Sized {
    /// Image sampler.
    type Sampler: 'static;

    /// Buffer type that can be used with this device.
    /// `UnboundedBuffer` can be converted to `Buffer` by `bind_buffer`.
    type Buffer: 'static;

    /// Unbounded buffer type that can be used with this device.
    /// `UnboundBuffer` hasn't been associated with memory yet.
    type UnboundBuffer: 'static;

    /// View to the buffer.
    type BufferView: 'static;

    /// Memory type that can be used with this device.
    /// `UnboundedImage` can be converted to `Image` by `bind_image`.
    type Image: 'static;

    /// Unbounded image type that can be used with this device.
    /// `UnboundImage` hasn't been associated with memory yet.
    type UnboundImage: 'static;

    /// View to the image.
    type ImageView: 'static;

    /// Create new unbound buffer object.
    fn create_buffer(
        &self,
        info: buffer::CreateInfo,
    ) -> Result<Self::UnboundBuffer, memory::OutOfMemoryError>;

    /// Fetch buffer memory requirements.
    fn buffer_requirements(&self, buffer: &Self::UnboundBuffer) -> MemoryRequirements;

    /// Bind memory range to the buffer.
    ///
    /// # Safety
    ///
    /// `offset` must be less than the size of memory.
    /// memory must have been allocated using one of the memory types allowed in the `mask` member of the `MemoryRequirements` structure returned from a call to `buffer_requirements` with buffer.
    /// `offset` must be an integer multiple of the alignment member of the `MemoryRequirements` structure returned from a call to `buffer_requirements` with buffer.
    /// The size member of the `MemoryRequirements` structure returned from a call to `buffer_requirements` with buffer must be less than or equal to the size of memory minus `offset`.
    unsafe fn bind_buffer(
        &self,
        buffer: Self::UnboundBuffer,
        memory: &Self::Memory,
        offset: u64,
    ) -> Result<Self::Buffer, error::BindError>;

    /// Destroy buffer object.
    unsafe fn destroy_buffer(&self, buffer: Self::Buffer);

    /// Create new unbound image object.
    fn create_image(
        &self,
        info: image::CreateInfo,
    ) -> Result<Self::UnboundImage, error::ImageCreationError>;

    /// Fetch image memory requirements.
    fn image_requirements(&self, image: &Self::UnboundImage) -> MemoryRequirements;

    /// Bind memory to the image.
    ///
    /// # Safety
    ///
    /// `offset` must be less than the size of memory.
    /// memory must have been allocated using one of the memory types allowed in the `mask` member of the `MemoryRequirements` structure returned from a call to `image_requirements` with image.
    /// `offset` must be an integer multiple of the alignment member of the `MemoryRequirements` structure returned from a call to `image_requirements` with image.
    /// The size member of the `MemoryRequirements` structure returned from a call to `image_requirements` with image must be less than or equal to the size of memory minus `offset`.
    unsafe fn bind_image(
        &self,
        image: Self::UnboundImage,
        memory: &Self::Memory,
        offset: u64,
    ) -> Result<Self::Image, error::BindError>;

    /// Destroy image object.
    unsafe fn destroy_image(&self, image: Self::Image);
}
