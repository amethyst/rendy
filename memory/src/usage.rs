//! Defines usage types for memory bocks.
//! See `Usage` and implementations for details.

use memory::Properties;

/// Memory usage trait.
pub trait Usage {
    /// Comparable fitness value.
    type Fitness: Copy + Ord;

    /// Get runtime usage value.
    fn value(self) -> UsageValue;

    /// Get comparable fitness value for memory properties.
    /// Should return `None` if memory doesn't fit.
    fn memory_fitness(&self, properties: Properties) -> Option<Self::Fitness>;
}

/// Full speed GPU access.
/// Optimal for render targets and persistent resources.
/// Avoid memory with host access.
#[derive(Clone, Copy, Debug)]
pub struct Data;

impl Usage for Data {
    type Fitness = u8;

    #[inline]
    fn value(self) -> UsageValue { UsageValue::Data }

    #[inline]
    fn memory_fitness(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::DEVICE_LOCAL) {
            None
        } else {
            Some(
                ((!properties.contains(Properties::HOST_VISIBLE)) as u8) << 3 |
                ((!properties.contains(Properties::LAZILY_ALLOCATED)) as u8) << 2 |
                ((!properties.contains(Properties::HOST_CACHED)) as u8) << 1 |
                ((!properties.contains(Properties::HOST_COHERENT)) as u8) << 0 |
                0
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

impl Usage for Dynamic {
    type Fitness = u8;

    #[inline]
    fn value(self) -> UsageValue { UsageValue::Dynamic }

    #[inline]
    fn memory_fitness(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::HOST_VISIBLE) {
            None
        } else {
            assert!(!properties.contains(Properties::LAZILY_ALLOCATED));
            Some(
                (properties.contains(Properties::DEVICE_LOCAL) as u8) << 2 |
                (properties.contains(Properties::HOST_COHERENT) as u8) << 1 |
                ((!properties.contains(Properties::HOST_CACHED)) as u8) << 0 |
                0
            )
        }
    }
}

/// CPU to GPU data flow with mapping.
/// Used for staging data before copying to the `Data` memory.
/// Host access is guaranteed.
#[derive(Clone, Copy, Debug)]
pub struct Upload;

impl Usage for Upload {
    type Fitness = u8;

    #[inline]
    fn value(self) -> UsageValue { UsageValue::Upload }

    #[inline]
    fn memory_fitness(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::HOST_VISIBLE) {
            None
        } else {
            assert!(!properties.contains(Properties::LAZILY_ALLOCATED));
            Some(
                ((!properties.contains(Properties::DEVICE_LOCAL)) as u8) << 2 |
                ((!properties.contains(Properties::HOST_CACHED)) as u8) << 0 |
                (properties.contains(Properties::HOST_COHERENT) as u8) << 1 |
                0
            )
        }
    }
}

/// GPU to CPU data flow with mapping.
/// Used for copying data from `Data` memory to be read by the host.
/// Host access is guaranteed.
#[derive(Clone, Copy, Debug)]
pub struct Download;

impl Usage for Download {
    type Fitness = u8;

    #[inline]
    fn value(self) -> UsageValue { UsageValue::Download }

    #[inline]
    fn memory_fitness(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::HOST_VISIBLE) {
            None
        } else {
            assert!(!properties.contains(Properties::LAZILY_ALLOCATED));
            Some(
                ((!properties.contains(Properties::DEVICE_LOCAL)) as u8) << 2 |
                (properties.contains(Properties::HOST_CACHED) as u8) << 1 |
                (properties.contains(Properties::HOST_COHERENT) as u8) << 0 |
                0
            )
        }
    }
}

/// Dynamic value that specify memory usage flags.
#[derive(Clone, Copy, Debug)]
pub enum UsageValue {
    /// Runtime counterpart for `Data`.
    Data,
    /// Runtime counterpart for `Dynamic`.
    Dynamic,
    /// Runtime counterpart for `Upload`.
    Upload,
    /// Runtime counterpart for `Download`.
    Download,
}

impl Usage for UsageValue {
    type Fitness = u8;

    #[inline]
    fn value(self) -> UsageValue { self }

    #[inline]
    fn memory_fitness(&self, properties: Properties) -> Option<u8> {
        match self {
            UsageValue::Data => Data.memory_fitness(properties),
            UsageValue::Dynamic => Dynamic.memory_fitness(properties),
            UsageValue::Upload => Upload.memory_fitness(properties),
            UsageValue::Download => Download.memory_fitness(properties),
        }
    }
}
