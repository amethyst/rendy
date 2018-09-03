
bitflags! {
    /// Bitmask specifying memory access types that will participate in a memory dependency.
    /// See Vulkan docs for detailed info:
    /// <https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkAccessFlagBits.html>
    #[repr(transparent)]
    pub struct AccessFlags: u32 {
        /// Access type performed by the device to read commands from indirect command buffer.
        const INDIRECT_COMMAND_READ = 0x00000001;

        /// Access type performed by the device to read from index buffer.
        const INDEX_READ = 0x00000002;

        /// Access type performed by the device to read from vertex attributes.
        const VERTEX_ATTRIBUTE_READ = 0x00000004;

        /// Access type performed by the device to read from uniform buffers.
        const UNIFORM_READ = 0x00000008;

        /// Access type performed by the device to read from input attachment.
        const INPUT_ATTACHMENT_READ = 0x00000010;

        /// Access type performed by the device to read from storage/uniform-texel/storage-texel buffers or sampled/storage images.
        const SHADER_READ = 0x00000020;

        /// Access type performed by the device to write to storage/storage-texel buffers or storage images.
        const SHADER_WRITE = 0x00000040;

        /// Access type performed by the device to read from color attachment.
        const COLOR_ATTACHMENT_READ = 0x00000080;

        /// Access type performed by the device to write to color attachment.
        const COLOR_ATTACHMENT_WRITE = 0x00000100;

        /// Access type performed by the device to read from depth-stencil attachment.
        const DEPTH_STENCIL_ATTACHMENT_READ = 0x00000200;

        /// Access type performed by the device to write to depth-stencil attachment.
        const DEPTH_STENCIL_ATTACHMENT_WRITE = 0x00000400;

        /// Access type performed by the device to read content from source of transfer operations.
        const TRANSFER_READ = 0x00000800;

        /// Access type performed by the device to write content to destination of transfer operations.
        const TRANSFER_WRITE = 0x00001000;

        /// Access type performed by host reading.
        const HOST_READ = 0x00002000;

        /// Access type performed by host writing.
        const HOST_WRITE = 0x00004000;

        /// Access type performed to read data via non-specific entities.
        const MEMORY_READ = 0x00008000;

        /// Access type performed to write data via non-specific entities.
        const MEMORY_WRITE = 0x00010000;
    }
}

impl AccessFlags {
    /// Check if flags contains at least on write flag.
    pub fn is_write(&self) -> bool {
        self.intersects(
            AccessFlags::SHADER_WRITE|
            AccessFlags::COLOR_ATTACHMENT_WRITE|
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE|
            AccessFlags::TRANSFER_WRITE|
            AccessFlags::HOST_WRITE|
            AccessFlags::MEMORY_WRITE
        )
    }

    /// Check if flags contains at least on read flag.
    pub fn is_read(&self) -> bool {
        self.intersects(
            AccessFlags::INDIRECT_COMMAND_READ|
            AccessFlags::INDEX_READ|
            AccessFlags::VERTEX_ATTRIBUTE_READ|
            AccessFlags::UNIFORM_READ|
            AccessFlags::INPUT_ATTACHMENT_READ|
            AccessFlags::SHADER_READ|
            AccessFlags::COLOR_ATTACHMENT_READ|
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ|
            AccessFlags::TRANSFER_READ|
            AccessFlags::HOST_READ|
            AccessFlags::MEMORY_READ
        )
    }
}
