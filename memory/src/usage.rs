//! Defines usage types for memory bocks.
//! See `Usage` and implementations for details.

use crate::allocator::Kind;

/// Memory usage trait.
pub trait MemoryUsage {
    /// Get set of properties required for the usage.
    fn properties_required(&self) -> gfx_hal::memory::Properties;

    /// Get comparable fitness value for memory properties.
    /// 
    /// # Panics
    /// 
    /// This function will panic if properties set doesn't contain required properties.
    fn memory_fitness(&self, properties: gfx_hal::memory::Properties) -> u32;

    /// Get comparable fitness value for memory allocator.
    fn allocator_fitness(&self, kind: Kind) -> u32;
}

/// Full speed GPU access.
/// Optimal for render targets and persistent resources.
/// Avoid memory with host access.
#[derive(Clone, Copy, Debug)]
pub struct Data;

impl MemoryUsage for Data {

    fn properties_required(&self) -> gfx_hal::memory::Properties {
        gfx_hal::memory::Properties::DEVICE_LOCAL
    }

    #[inline]
    fn memory_fitness(&self, properties: gfx_hal::memory::Properties) -> u32 {
        assert!(properties.contains(gfx_hal::memory::Properties::DEVICE_LOCAL));
        0 | ((!properties.contains(gfx_hal::memory::Properties::CPU_VISIBLE)) as u32) << 3
          | ((!properties.contains(gfx_hal::memory::Properties::LAZILY_ALLOCATED)) as u32) << 2
          | ((!properties.contains(gfx_hal::memory::Properties::CPU_CACHED)) as u32) << 1
          | ((!properties.contains(gfx_hal::memory::Properties::COHERENT)) as u32) << 0
    }

    fn allocator_fitness(&self, kind: Kind) -> u32 {
        match kind {
            Kind::Dedicated => 1,
            Kind::Dynamic => 2,
            Kind::Linear => 0,
        }
    }
}

/// CPU to GPU data flow with update commands.
/// Used for dynamic buffer data, typically constant buffers.
/// Host access is guaranteed.
/// Prefers memory with fast GPU access.
#[derive(Clone, Copy, Debug)]
pub struct Dynamic;

impl MemoryUsage for Dynamic {

    fn properties_required(&self) -> gfx_hal::memory::Properties {
        gfx_hal::memory::Properties::CPU_VISIBLE
    }

    #[inline]
    fn memory_fitness(&self, properties: gfx_hal::memory::Properties) -> u32 {
        assert!(properties.contains(gfx_hal::memory::Properties::CPU_VISIBLE));
        assert!(!properties.contains(gfx_hal::memory::Properties::LAZILY_ALLOCATED));
    
        0 | (properties.contains(gfx_hal::memory::Properties::DEVICE_LOCAL) as u32) << 2
          | (properties.contains(gfx_hal::memory::Properties::COHERENT) as u32) << 1
          | ((!properties.contains(gfx_hal::memory::Properties::CPU_CACHED)) as u32) << 0
    }

    fn allocator_fitness(&self, kind: Kind) -> u32 {
        match kind {
            Kind::Dedicated => 1,
            Kind::Dynamic => 2,
            Kind::Linear => 0,
        }
    }
}

/// CPU to GPU data flow with mapping.
/// Used for staging data before copying to the `Data` memory.
/// Host access is guaranteed.
#[derive(Clone, Copy, Debug)]
pub struct Upload;

impl MemoryUsage for Upload {

    fn properties_required(&self) -> gfx_hal::memory::Properties {
        gfx_hal::memory::Properties::CPU_VISIBLE
    }

    #[inline]
    fn memory_fitness(&self, properties: gfx_hal::memory::Properties) -> u32 {
        assert!(properties.contains(gfx_hal::memory::Properties::CPU_VISIBLE));
        assert!(!properties.contains(gfx_hal::memory::Properties::LAZILY_ALLOCATED));

        0 | ((!properties.contains(gfx_hal::memory::Properties::DEVICE_LOCAL)) as u32) << 2
          | (properties.contains(gfx_hal::memory::Properties::COHERENT) as u32) << 1
          | ((!properties.contains(gfx_hal::memory::Properties::CPU_CACHED)) as u32) << 0
    }

    fn allocator_fitness(&self, kind: Kind) -> u32 {
        match kind {
            Kind::Dedicated => 0,
            Kind::Dynamic => 1,
            Kind::Linear => 2,
        }
    }
}

/// GPU to CPU data flow with mapping.
/// Used for copying data from `Data` memory to be read by the host.
/// Host access is guaranteed.
#[derive(Clone, Copy, Debug)]
pub struct Download;

impl MemoryUsage for Download {

    fn properties_required(&self) -> gfx_hal::memory::Properties {
        gfx_hal::memory::Properties::CPU_VISIBLE
    }

    #[inline]
    fn memory_fitness(&self, properties: gfx_hal::memory::Properties) -> u32 {
        assert!(properties.contains(gfx_hal::memory::Properties::CPU_VISIBLE));
        assert!(!properties.contains(gfx_hal::memory::Properties::LAZILY_ALLOCATED));

        0 | ((!properties.contains(gfx_hal::memory::Properties::DEVICE_LOCAL)) as u32) << 2
          | (properties.contains(gfx_hal::memory::Properties::CPU_CACHED) as u32) << 1
          | (properties.contains(gfx_hal::memory::Properties::COHERENT) as u32) << 0
    }

    fn allocator_fitness(&self, kind: Kind) -> u32 {
        match kind {
            Kind::Dedicated => 0,
            Kind::Dynamic => 1,
            Kind::Linear => 2,
        }
    }
}

