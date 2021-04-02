use rendy_core::hal;
use super::sync::SyncPoint;

mod format;
pub use format::{PartialFormat, NumericFormat, DepthStencilComponents};

/// A transient image is a framebuffer attachment used in a single render pass.
/// They begin in a cleared or uninitialized state, and are read and written by
/// several subpasses before being discarded in the end. They carry major
/// advantages on tiled renderers where they may not need to be backed by system
/// memory. If all usages of a transient image cannot be combined into a single
/// render pass, a graph validation warning will be emitted, and a regular image
/// will be used instead.
#[derive(Debug, Copy, Clone)]
pub enum ImageMode {
    /// Use image contents from the previous frame.
    /// When the image is used for the first time in a series of frames, the clear
    /// operation is used instead. Specify a `token` to link the contents across
    /// frames.
    ///
    /// Note that the backing contents of a persistance tokens will be kept around
    /// until either:
    /// 1. The whole graph is dropped
    /// 2. The persistance token is disposed of
    /// Beware of leaking memory.
    ///
    /// Creating an image using the same persistance token more than once in a
    /// single graph run will result in a graph validation error.
    Retain {
        token: usize,
        clear: hal::command::ClearValue,
    },
    /// Image contents left undefined. Fastest option if you expect to overwrite
    /// it all anyway.
    DontCare,
    /// Clear the image with the specified image when using it for the first
    /// time.
    Clear {
        clear: hal::command::ClearValue,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct ImageInfo {
    /// If the dimensions is None, this is inferred by the graph.
    pub kind: Option<hal::image::Kind>,
    pub levels: hal::image::Level,
    pub format: hal::format::Format,
    pub mode: ImageMode,
}

#[derive(Debug, Copy, Clone)]
pub struct BufferInfo {
    pub size: u64,
    pub clear: Option<u32>,
}

/// TODO: Better abstraction
#[derive(Debug, Copy, Clone)]
pub struct ImageUsage {
    /// The image layout this image should be used as.
    /// `Layout::General` is completely general, and is a safe default,
    /// but may be more inefficient than more specific layouts.
    pub layout: hal::image::Layout,

    /// The stage(s) of the pipeline this image is used in.
    /// This is REQUIRED to be set to ALL stages the image is accessed in.
    /// Not doing so can lead to corrupted results.
    /// `PipelineStage::all()` is a safe default, but may impact performance.
    /// TODO: The graph filters out unsupported flags for the queue, but maybe
    /// this could be made more statically safe in abstraction?
    pub stages: hal::pso::PipelineStage,

    /// The types of read and write access done to the image.
    /// This is REQUIRED to be set for all reads and writes done to the image.
    /// Not doing so can lead to corrupted results.
    /// `Access::all()` is a safe default, but may impact performance.
    /// TODO: The graph filters out unsupported flags for the queue, but maybe
    /// this could be made more statically safe in abstraction?
    pub access: hal::image::Access,

    // TODO add ImageSubResourceRange
}

impl ImageUsage {

    pub fn is_read(&self) -> bool {
        use hal::image::Access as A;
        self.access.intersects(
            A::INPUT_ATTACHMENT_READ | A::SHADER_READ | A::COLOR_ATTACHMENT_READ
                | A::DEPTH_STENCIL_ATTACHMENT_READ | A::TRANSFER_READ
                | A::HOST_READ | A::MEMORY_READ
        )
    }

    pub fn is_write(&self) -> bool {
        use hal::image::Access as A;
        self.access.intersects(
            A::SHADER_WRITE | A::COLOR_ATTACHMENT_WRITE | A::DEPTH_STENCIL_ATTACHMENT_WRITE
                | A::TRANSFER_WRITE | A::HOST_WRITE | A::MEMORY_WRITE
        )
    }

}

impl Default for ImageUsage {
    fn default() -> Self {
        ImageUsage {
            layout: hal::image::Layout::General,
            stages: hal::pso::PipelineStage::all(),
            access: hal::image::Access::all(),
        }
    }
}

/// TODO: Better abstraction
#[derive(Debug, Copy, Clone)]
pub struct BufferUsage {
    /// The stage(s) of the pipeline this buffer is used in.
    /// This is REQUIRED to be set to ALL stages the buffer is accessed in.
    /// Not doing so can lead to corrupted results.
    /// `PipelineStage::all()` is a safe default, but may impact performance.
    /// TODO: The graph filters out unsupported flags for the queue, but maybe
    /// this could be made more statically safe in abstraction?
    pub stages: hal::pso::PipelineStage,

    /// The types of read and write access done to the buffer.
    /// This is REQUIRED to be set for all reads and writes done to the buffer.
    /// Not doing so can lead to corrupted results.
    /// `Access::all()` is a safe default, but may impact performance.
    /// TODO: The graph filters out unsupported flags for the queue, but maybe
    /// this could be made more statically safe in abstraction?
    pub access: hal::buffer::Access,

    /// If `None`, access is assumed for the whole buffer.
    /// Is `Some(offset, size)`, access is assumed for only that part of the
    /// buffer.
    pub region: Option<(usize, usize)>,
}

impl BufferUsage {

    pub fn is_read(&self) -> bool {
        use hal::buffer::Access as A;
        self.access.intersects(
            A::INDIRECT_COMMAND_READ | A::INDEX_BUFFER_READ | A::VERTEX_BUFFER_READ
                | A::UNIFORM_READ | A::SHADER_READ | A::TRANSFER_READ
                | A::HOST_READ | A::MEMORY_READ
        )
    }

    pub fn is_write(&self) -> bool {
        use hal::buffer::Access as A;
        self.access.intersects(
            A::SHADER_WRITE | A::TRANSFER_WRITE | A::HOST_WRITE | A::MEMORY_WRITE
        )
    }

}

impl Default for BufferUsage {
    fn default() -> Self {
        BufferUsage {
            stages: hal::pso::PipelineStage::all(),
            access: hal::buffer::Access::all(),
            region: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ProvidedImageUsage {
    /// If a layout is supplied, the image will be assumed to be in this Layout
    /// when provided.
    pub layout: hal::image::Layout,

    /// If last_access is provided, the the graph will know this image needs
    /// a memory barrier on the given accesses before the image can be accessed.
    pub last_access: hal::image::Access,
}

#[derive(Debug, Copy, Clone)]
pub struct ProvidedBufferUsage {
    /// If last_access is provided, the the graph will know this buffer needs
    /// a memory barrier on the given accesses before the buffer can be accessed.
    pub last_access: hal::buffer::Access,
}

#[derive(Debug, Copy, Clone)]
pub enum VirtualUsage {
    /// Concurrent access with other reads.
    Read,
    /// Exclusive access.
    Write,
}
