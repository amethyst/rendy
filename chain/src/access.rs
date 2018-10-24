
use ash::vk::AccessFlags;

/// Add methods to fetch higher-level info from access flags.
pub trait AccessFlagsExt {
    /// Check if this access must be exclusive.
    /// 
    /// Basically this checks if all flags are known read flags.
    fn exclusive(&self) -> bool;
}

#[inline]
#[allow(unused)]
fn known_flags() -> AccessFlags { AccessFlags::INDIRECT_COMMAND_READ
    | AccessFlags::INDEX_READ
    | AccessFlags::VERTEX_ATTRIBUTE_READ
    | AccessFlags::UNIFORM_READ
    | AccessFlags::INPUT_ATTACHMENT_READ
    | AccessFlags::SHADER_READ
    | AccessFlags::SHADER_WRITE
    | AccessFlags::COLOR_ATTACHMENT_READ
    | AccessFlags::COLOR_ATTACHMENT_WRITE
    | AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
    | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
    | AccessFlags::TRANSFER_READ
    | AccessFlags::TRANSFER_WRITE
    | AccessFlags::HOST_READ
    | AccessFlags::HOST_WRITE
    | AccessFlags::MEMORY_READ
    | AccessFlags::MEMORY_WRITE
    // | AccessFlags::TRANSFORM_FEEDBACK_WRITE_EXT
    // | AccessFlags::TRANSFORM_FEEDBACK_COUNTER_READ_EXT
    // | AccessFlags::TRANSFORM_FEEDBACK_COUNTER_WRITE_EXT
    // | AccessFlags::CONDITIONAL_RENDERING_READ_EXT
    | AccessFlags::COMMAND_PROCESS_READ_NVX
    | AccessFlags::COMMAND_PROCESS_WRITE_NVX
    | AccessFlags::COLOR_ATTACHMENT_READ_NONCOHERENT_EXT
    // | AccessFlags::SHADING_RATE_IMAGE_READ_NV
    // | AccessFlags::ACCELERATION_STRUCTURE_READ_NVX
    // | AccessFlags::ACCELERATION_STRUCTURE_WRITE_NVX
}

#[inline]
#[allow(unused)]
fn write_flags() -> AccessFlags { AccessFlags::SHADER_WRITE
    | AccessFlags::COLOR_ATTACHMENT_WRITE
    | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
    | AccessFlags::TRANSFER_WRITE
    | AccessFlags::HOST_WRITE
    | AccessFlags::MEMORY_WRITE
    // | AccessFlags::TRANSFORM_FEEDBACK_WRITE_EXT
    // | AccessFlags::TRANSFORM_FEEDBACK_COUNTER_WRITE_EXT
    | AccessFlags::COMMAND_PROCESS_WRITE_NVX
    // | AccessFlags::ACCELERATION_STRUCTURE_WRITE_NVX
}

#[inline]
fn read_flags() -> AccessFlags { AccessFlags::INDIRECT_COMMAND_READ
    | AccessFlags::INDEX_READ
    | AccessFlags::VERTEX_ATTRIBUTE_READ
    | AccessFlags::UNIFORM_READ
    | AccessFlags::INPUT_ATTACHMENT_READ
    | AccessFlags::SHADER_READ
    | AccessFlags::COLOR_ATTACHMENT_READ
    | AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
    | AccessFlags::TRANSFER_READ
    | AccessFlags::HOST_READ
    | AccessFlags::MEMORY_READ
    // | AccessFlags::TRANSFORM_FEEDBACK_COUNTER_READ_EXT
    // | AccessFlags::CONDITIONAL_RENDERING_READ_EXT
    | AccessFlags::COMMAND_PROCESS_READ_NVX
    | AccessFlags::COLOR_ATTACHMENT_READ_NONCOHERENT_EXT
    // | AccessFlags::SHADING_RATE_IMAGE_READ_NV
    // | AccessFlags::ACCELERATION_STRUCTURE_READ_NVX
}

impl AccessFlagsExt for AccessFlags {
    #[inline]
    fn exclusive(&self) -> bool {
        read_flags().subset(*self)
    }
}
