use ash::vk::BufferUsageFlags;
use memory::usage::{Data, Download, Dynamic, MemoryUsage, MemoryUsageValue, Upload};

/// Usage trait that must implemented by usage types.
/// This trait provides a way to convert type-level usage to the value-level flags.
pub trait Usage {
    /// Suggested memory usage type.
    type MemoryUsage: MemoryUsage;

    /// Convert usage to the flags.
    fn flags(&self) -> BufferUsageFlags;

    /// Get suggested memory usage.
    fn memory(&self) -> Self::MemoryUsage;
}

impl Usage for (BufferUsageFlags, MemoryUsageValue) {
    type MemoryUsage = MemoryUsageValue;

    fn flags(&self) -> BufferUsageFlags {
        self.0
    }

    fn memory(&self) -> MemoryUsageValue {
        self.1
    }
}

/// Type that specify that buffer is intended to be used as vertex buffer.
/// It implies `TRANSFER_DST` because device-local, host-invisible memory should be used
/// and transfer is left the only way to fill the buffer.
#[derive(Clone, Copy, Debug)]
pub struct VertexBuffer;

impl Usage for VertexBuffer {
    type MemoryUsage = Data;

    fn flags(&self) -> BufferUsageFlags {
        BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::VERTEX_BUFFER
    }

    fn memory(&self) -> Data {
        Data
    }
}

/// Type that specify that buffer is intended to be used as index buffer.
/// It implies `TRANSFER_DST` because device-local, host-invisible memory should be used
/// and transfer is left the only way to fill the buffer.
#[derive(Clone, Copy, Debug)]
pub struct IndexBuffer;

impl Usage for IndexBuffer {
    type MemoryUsage = Data;

    fn flags(&self) -> BufferUsageFlags {
        BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::INDEX_BUFFER
    }

    fn memory(&self) -> Data {
        Data
    }
}

/// Type that specify that buffer is intended to be used as uniform buffer.
/// Host visible memory required and device-local preferred.
#[derive(Clone, Copy, Debug)]
pub struct UniformBuffer;

impl Usage for UniformBuffer {
    type MemoryUsage = Dynamic;

    fn flags(&self) -> BufferUsageFlags {
        BufferUsageFlags::UNIFORM_BUFFER
    }

    fn memory(&self) -> Dynamic {
        Dynamic
    }
}

/// Type that specify that buffer is intended to be used as staging buffer for data uploads.
#[derive(Clone, Copy, Debug)]
pub struct UploadBuffer;

impl Usage for UploadBuffer {
    type MemoryUsage = Upload;

    fn flags(&self) -> BufferUsageFlags {
        BufferUsageFlags::TRANSFER_SRC
    }

    fn memory(&self) -> Upload {
        Upload
    }
}

/// Type that specify that buffer is intended to be used as staging buffer for data downloads.
#[derive(Clone, Copy, Debug)]
pub struct DownloadBuffer;

impl Usage for DownloadBuffer {
    type MemoryUsage = Download;

    fn flags(&self) -> BufferUsageFlags {
        BufferUsageFlags::TRANSFER_DST
    }

    fn memory(&self) -> Download {
        Download
    }
}
