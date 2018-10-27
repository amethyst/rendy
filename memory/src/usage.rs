//! Defines usage types for memory bocks.
//! See `Usage` and implementations for details.

use ash::vk::MemoryPropertyFlags;

/// Memory usage trait.
pub trait MemoryUsage {
    /// Comparable fitness value.
    type Fitness: Copy + Ord;

    /// Get runtime usage value.
    fn value(self) -> MemoryUsageValue;

    /// Get comparable fitness value for memory properties.
    /// Should return `None` if memory doesn't fit.
    fn memory_fitness(&self, properties: MemoryPropertyFlags) -> Option<Self::Fitness>;
}

/// Full speed GPU access.
/// Optimal for render targets and persistent resources.
/// Avoid memory with host access.
#[derive(Clone, Copy, Debug)]
pub struct Data;

impl MemoryUsage for Data {
    type Fitness = u8;

    #[inline]
    fn value(self) -> MemoryUsageValue {
        MemoryUsageValue::Data
    }

    #[inline]
    fn memory_fitness(&self, properties: MemoryPropertyFlags) -> Option<u8> {
        if !properties.subset(MemoryPropertyFlags::DEVICE_LOCAL) {
            None
        } else {
            Some(
                ((!properties.subset(MemoryPropertyFlags::HOST_VISIBLE)) as u8) << 3
                    | ((!properties.subset(MemoryPropertyFlags::LAZILY_ALLOCATED)) as u8) << 2
                    | ((!properties.subset(MemoryPropertyFlags::HOST_CACHED)) as u8) << 1
                    | ((!properties.subset(MemoryPropertyFlags::HOST_COHERENT)) as u8) << 0
                    | 0,
            )
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
    type Fitness = u8;

    #[inline]
    fn value(self) -> MemoryUsageValue {
        MemoryUsageValue::Dynamic
    }

    #[inline]
    fn memory_fitness(&self, properties: MemoryPropertyFlags) -> Option<u8> {
        if !properties.subset(MemoryPropertyFlags::HOST_VISIBLE) {
            None
        } else {
            assert!(!properties.subset(MemoryPropertyFlags::LAZILY_ALLOCATED));
            Some(
                (properties.subset(MemoryPropertyFlags::DEVICE_LOCAL) as u8) << 2
                    | (properties.subset(MemoryPropertyFlags::HOST_COHERENT) as u8) << 1
                    | ((!properties.subset(MemoryPropertyFlags::HOST_CACHED)) as u8) << 0
                    | 0,
            )
        }
    }
}

/// CPU to GPU data flow with mapping.
/// Used for staging data before copying to the `Data` memory.
/// Host access is guaranteed.
#[derive(Clone, Copy, Debug)]
pub struct Upload;

impl MemoryUsage for Upload {
    type Fitness = u8;

    #[inline]
    fn value(self) -> MemoryUsageValue {
        MemoryUsageValue::Upload
    }

    #[inline]
    fn memory_fitness(&self, properties: MemoryPropertyFlags) -> Option<u8> {
        if !properties.subset(MemoryPropertyFlags::HOST_VISIBLE) {
            None
        } else {
            assert!(!properties.subset(MemoryPropertyFlags::LAZILY_ALLOCATED));
            Some(
                ((!properties.subset(MemoryPropertyFlags::DEVICE_LOCAL)) as u8) << 2
                    | ((!properties.subset(MemoryPropertyFlags::HOST_CACHED)) as u8) << 0
                    | (properties.subset(MemoryPropertyFlags::HOST_COHERENT) as u8) << 1
                    | 0,
            )
        }
    }
}

/// GPU to CPU data flow with mapping.
/// Used for copying data from `Data` memory to be read by the host.
/// Host access is guaranteed.
#[derive(Clone, Copy, Debug)]
pub struct Download;

impl MemoryUsage for Download {
    type Fitness = u8;

    #[inline]
    fn value(self) -> MemoryUsageValue {
        MemoryUsageValue::Download
    }

    #[inline]
    fn memory_fitness(&self, properties: MemoryPropertyFlags) -> Option<u8> {
        if !properties.subset(MemoryPropertyFlags::HOST_VISIBLE) {
            None
        } else {
            assert!(!properties.subset(MemoryPropertyFlags::LAZILY_ALLOCATED));
            Some(
                ((!properties.subset(MemoryPropertyFlags::DEVICE_LOCAL)) as u8) << 2
                    | (properties.subset(MemoryPropertyFlags::HOST_CACHED) as u8) << 1
                    | (properties.subset(MemoryPropertyFlags::HOST_COHERENT) as u8) << 0
                    | 0,
            )
        }
    }
}

/// Dynamic value that specify memory usage flags.
#[derive(Clone, Copy, Debug)]
pub enum MemoryUsageValue {
    /// Runtime counterpart for `Data`.
    Data,
    /// Runtime counterpart for `Dynamic`.
    Dynamic,
    /// Runtime counterpart for `Upload`.
    Upload,
    /// Runtime counterpart for `Download`.
    Download,
}

impl MemoryUsage for MemoryUsageValue {
    type Fitness = u8;

    #[inline]
    fn value(self) -> MemoryUsageValue {
        self
    }

    #[inline]
    fn memory_fitness(&self, properties: MemoryPropertyFlags) -> Option<u8> {
        match self {
            MemoryUsageValue::Data => Data.memory_fitness(properties),
            MemoryUsageValue::Dynamic => Dynamic.memory_fitness(properties),
            MemoryUsageValue::Upload => Upload.memory_fitness(properties),
            MemoryUsageValue::Download => Download.memory_fitness(properties),
        }
    }
}
