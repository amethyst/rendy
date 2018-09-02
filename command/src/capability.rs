


bitflags! {
    /// Bitmask specifying capabilities of queues in a queue family.
    /// See Vulkan docs for detailed info:
    /// https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkQueueFlagBits.html
    #[repr(transparent)]
    pub struct CapabilityFlags: u32 {
        const GRAPHICS = 0x00000001;
        const COMPUTE = 0x00000002;
        const TRANSFER = 0x00000004;
        const SPARSE_BINDING = 0x00000008;
        const PROTECTED = 0x00000010;
    }
}


impl Capability {
    /// Get capabilities required by pipeline stages.
    pub fn required_queue_capability(self) -> Capability {
        let mut capability = Capability::empty();
        if (self.contains(PipelineStageFlags::DRAW_INDIRECT) { capability |= Capability::GRAPHICS | Capability::COMPUTE; }
        if (self.contains(PipelineStageFlags::VERTEX_INPUT) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::VERTEX_SHADER) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::TESSELLATION_CONTROL_SHADER) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::TESSELLATION_EVALUATION_SHADER) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::GEOMETRY_SHADER) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::FRAGMENT_SHADER) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::EARLY_FRAGMENT_TESTS) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::LATE_FRAGMENT_TESTS) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT) { capability |= Capability::GRAPHICS; }
        if (self.contains(PipelineStageFlags::COMPUTE_SHADER) { capability |= Capability::COMPUTE; }
        if (self.contains(PipelineStageFlags::TRANSFER) { capability |= Capability::GRAPHICS | Capability::COMPUTE | Capability::TRANSFER; }
        if (self.contains(PipelineStageFlags::ALL_GRAPHICS) { capability |= Capability::GRAPHICS; }
        capability
    }
}

use capability::Capability;
