
bitflags! {
    /// Bitmask specifying allowed usage of a buffer.
    /// See Vulkan docs for detailed info:
    /// https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkBufferUsageFlagBits.html
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

pub trait Usage {
    fn flags(&self) -> UsageFlags;
}

#[derive(Debug)]
pub struct UsageValue(UsageFlags);

impl Usage for UsageValue {
    fn flags(&self) -> UsageFlags {
        self.0
    }
}

#[derive(Debug)]
pub struct VertexBuffer;

impl Usage for VertexBuffer {
    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_DST | UsageFlags::VERTEX_BUFFER
    }
}

#[derive(Debug)]
pub struct IndexBuffer;

impl Usage for IndexBuffer {
    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_DST | UsageFlags::INDEX_BUFFER
    }
}

#[derive(Debug)]
pub struct UniformBuffer;

impl Usage for UniformBuffer {
    fn flags(&self) -> UsageFlags {
        UsageFlags::UNIFORM_BUFFER
    }
}

#[derive(Debug)]
pub struct UploadBuffer;

impl Usage for UploadBuffer {
    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_SRC
    }
}

#[derive(Debug)]
pub struct DownloadBuffer;

impl Usage for DownloadBuffer {
    fn flags(&self) -> UsageFlags {
        UsageFlags::TRANSFER_DST
    }
}
