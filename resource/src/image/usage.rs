
use ash::vk::ImageUsageFlags;
use memory::usage::{Data, Usage as MemoryUsage, UsageValue};

/// Usage trait that must implemented by usage types.
/// This trait provides a way to convert type-level usage to the value-level flags.
pub trait Usage {
    /// Suggested memory usage type.
    type MemoryUsage: MemoryUsage;

    /// Convert usage to the flags.
    fn flags(&self) -> ImageUsageFlags;

    /// Get suggested memory usage.
    fn memory(&self) -> Self::MemoryUsage;
}

impl Usage for (ImageUsageFlags, UsageValue) {
    type MemoryUsage = UsageValue;

    fn flags(&self) -> ImageUsageFlags {
        self.0
    }

    fn memory(&self) -> UsageValue {
        self.1
    }
}

/// Type that specify that image is intended to be used as texture.
/// It implies `TRANSFER_DST` because device-local, host-invisible memory should be used
/// and transfer is left the only way to fill the buffer.
#[derive(Clone, Copy, Debug)]
pub struct Texture;

impl Usage for Texture {
    type MemoryUsage = Data;

    fn flags(&self) -> ImageUsageFlags {
        ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED
    }

    fn memory(&self) -> Data {
        Data
    }
}

/// Type that specify that image is intended to be used as render target and storage image.
#[derive(Clone, Copy, Debug)]
pub struct RenderTargetStorage;

impl Usage for RenderTargetStorage {
    type MemoryUsage = Data;

    fn flags(&self) -> ImageUsageFlags {
        ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::STORAGE
    }

    fn memory(&self) -> Data {
        Data
    }
}

/// Type that specify that image is intended to be used as render target and sampled image.
#[derive(Clone, Copy, Debug)]
pub struct RenderTargetSampled;

impl Usage for RenderTargetSampled {
    type MemoryUsage = Data;

    fn flags(&self) -> ImageUsageFlags {
        ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED
    }

    fn memory(&self) -> Data {
        Data
    }
}
