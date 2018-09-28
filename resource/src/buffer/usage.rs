use memory::usage::{Data, Download, Dynamic, Upload, Usage as MemoryUsage, UsageValue};

bitflags! {
    /// Bitmask specifying allowed usage of a buffer.
    /// See Vulkan docs for detailed info:
    /// <https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkBufferUsageFlagBits.html>
    #[repr(transparent)]
    pub struct UsageFlags: u32 {
        /// Specifies that the buffer can be used as the source of a transfer command.
        const TRANSFER_SRC = 0x00000001;

        /// Specifies that the buffer can be used as the destination of a transfer command.
        const TRANSFER_DST = 0x00000002;

        /// Specifies that the buffer can be used to create a `BufferView` suitable for occupying a descriptor set slot of type `UNIFORM_TEXEL_BUFFER`.
        const UNIFORM_TEXEL_BUFFER = 0x00000004;

        /// Specifies that the buffer can be used to create a `BufferView` suitable for occupying a descriptor set slot of type `STORAGE_TEXEL_BUFFER`.
        const STORAGE_TEXEL_BUFFER = 0x00000008;

        /// Specifies that the buffer can be used in a descriptor buffer info suitable for occupying a descriptor set slot either of
        /// type `UNIFORM_BUFFER` or `UNIFORM_BUFFER_DYNAMIC`.
        const UNIFORM_BUFFER = 0x00000010;

        /// Specifies that the buffer can be used in a descriptor buffer info suitable for occupying a descriptor set slot either of
        /// type `STORAGE_BUFFER` or `STORAGE_BUFFER_DYNAMIC`.
        const STORAGE_BUFFER = 0x00000020;

        /// Specifies that the buffer is suitable for vertex indices.
        const INDEX_BUFFER = 0x00000040;

        /// Specifies that the buffer is suitable for vertex attributes.
        const VERTEX_BUFFER = 0x00000080;

        /// Specifies that the buffer is suitable for indirect commands.
        const INDIRECT_BUFFER = 0x00000100;
    }
}

/// Usage trait that must implemented by usage types.
/// This trait provides a way to convert type-level usage to the value-level flags.
pub trait Usage {
    /// Suggested memory usage type.
    type MemoryUsage: MemoryUsage;

    /// Convert usage to the flags.
    fn flags(&self) -> UsageFlags;

    /// Get suggested memory usage.
    fn memory(&self) -> Self::MemoryUsage;
}

impl Usage for (UsageFlags, UsageValue) {
    type MemoryUsage = UsageValue;

    fn flags(&self) -> UsageFlags {
        self.0
    }

    fn memory(&self) -> UsageValue {
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

    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_DST | UsageFlags::VERTEX_BUFFER
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

    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_DST | UsageFlags::INDEX_BUFFER
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

    fn flags(&self) -> UsageFlags {
        UsageFlags::UNIFORM_BUFFER
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

    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_SRC
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

    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_DST
    }

    fn memory(&self) -> Download {
        Download
    }
}
