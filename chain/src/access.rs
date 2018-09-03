
bitflags! {
    /// Bitmask specifying memory access types that will participate in a memory dependency.
    /// See Vulkan docs for detailed info:
    /// <https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkAccessFlagBits.html>
    #[repr(transparent)]
    pub struct AccessFlags: u32 {
        const INDIRECT_COMMAND_READ = 0x00000001;
        const INDEX_READ = 0x00000002;
        const VERTEX_ATTRIBUTE_READ = 0x00000004;
        const UNIFORM_READ = 0x00000008;
        const INPUT_ATTACHMENT_READ = 0x00000010;
        const SHADER_READ = 0x00000020;
        const SHADER_WRITE = 0x00000040;
        const COLOR_ATTACHMENT_READ = 0x00000080;
        const COLOR_ATTACHMENT_WRITE = 0x00000100;
        const DEPTH_STENCIL_ATTACHMENT_READ = 0x00000200;
        const DEPTH_STENCIL_ATTACHMENT_WRITE = 0x00000400;
        const TRANSFER_READ = 0x00000800;
        const TRANSFER_WRITE = 0x00001000;
        const HOST_READ = 0x00002000;
        const HOST_WRITE = 0x00004000;
        const MEMORY_READ = 0x00008000;
        const MEMORY_WRITE = 0x00010000;
    }
}

impl AccessFlags {
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
