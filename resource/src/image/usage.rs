use crate::memory::{Data, MemoryUsage};

/// Usage trait that must implemented by usage types.
/// This trait provides a way to convert type-level usage to the value-level flags.
pub trait Usage: std::fmt::Debug {
    /// Suggested memory usage type.
    type MemoryUsage: MemoryUsage;

    /// Convert usage to the flags.
    fn flags(&self) -> gfx_hal::image::Usage;

    /// Get suggested memory usage.
    fn memory(&self) -> Self::MemoryUsage;
}

impl<M> Usage for (gfx_hal::image::Usage, M)
where
    M: MemoryUsage,
{
    type MemoryUsage = M;

    fn flags(&self) -> gfx_hal::image::Usage {
        self.0
    }

    fn memory(&self) -> M {
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

    fn flags(&self) -> gfx_hal::image::Usage {
        gfx_hal::image::Usage::TRANSFER_DST | gfx_hal::image::Usage::SAMPLED
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

    fn flags(&self) -> gfx_hal::image::Usage {
        gfx_hal::image::Usage::COLOR_ATTACHMENT | gfx_hal::image::Usage::STORAGE
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

    fn flags(&self) -> gfx_hal::image::Usage {
        gfx_hal::image::Usage::COLOR_ATTACHMENT | gfx_hal::image::Usage::SAMPLED
    }

    fn memory(&self) -> Data {
        Data
    }
}
