
use crate::memory::{Data, Download, Dynamic, MemoryUsage, Upload};

/// Usage trait that must implemented by usage types.
/// This trait provides a way to convert type-level usage to the value-level flags.
pub trait Usage: std::fmt::Debug {
    /// Suggested memory usage type.
    type MemoryUsage: MemoryUsage;

    /// Convert usage to the flags.
    fn flags(&self) -> gfx_hal::buffer::Usage;

    /// Get suggested memory usage.
    fn memory(&self) -> Self::MemoryUsage;
}

impl<M> Usage for (gfx_hal::buffer::Usage, M)
where
    M: MemoryUsage,
{
    type MemoryUsage = M;

    fn flags(&self) -> gfx_hal::buffer::Usage {
        self.0
    }

    fn memory(&self) -> M {
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

    fn flags(&self) -> gfx_hal::buffer::Usage {
        gfx_hal::buffer::Usage::TRANSFER_DST | gfx_hal::buffer::Usage::VERTEX
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

    fn flags(&self) -> gfx_hal::buffer::Usage {
        gfx_hal::buffer::Usage::TRANSFER_DST | gfx_hal::buffer::Usage::INDEX
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

    fn flags(&self) -> gfx_hal::buffer::Usage {
        gfx_hal::buffer::Usage::UNIFORM
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

    fn flags(&self) -> gfx_hal::buffer::Usage {
        gfx_hal::buffer::Usage::TRANSFER_SRC
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

    fn flags(&self) -> gfx_hal::buffer::Usage {
        gfx_hal::buffer::Usage::TRANSFER_DST
    }

    fn memory(&self) -> Download {
        Download
    }
}
