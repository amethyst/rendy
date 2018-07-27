
use hal::memory::Properties;

/// Memory usage trait.
pub trait Usage {
    type Key: Copy + Ord;
    /// Get comparable key for memory properties.
    /// Should return `None` if memory not fit.
    fn key(&self, properties: Properties) -> Option<Self::Key>;
}

/// Full speed GPU access.
/// Optimal for render targets and resourced memory.
/// Requires `DEVICE_LOCAL` memory.
pub struct Data;

impl Usage for Data {
    type Key = u8;

    fn key(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::DEVICE_LOCAL) {
            None
        } else {
            Some(
                ((!properties.contains(Properties::CPU_VISIBLE)) as u8) << 3 |
                ((!properties.contains(Properties::LAZILY_ALLOCATED)) as u8) << 2 |
                ((!properties.contains(Properties::CPU_CACHED)) as u8) << 1 |
                ((!properties.contains(Properties::COHERENT)) as u8) << 0 |
                0
            )
        }
    }
}

/// CPU to GPU data flow with update commands.
/// Used for dynamic buffer data, typically constant buffers.
/// Requires `HOST_VISIBLE` memory with `DEVICE_LOCAL` preferable.
pub struct Dynamic;

impl Usage for Dynamic {
    type Key = u8;

    fn key(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::CPU_VISIBLE) {
            None
        } else {
            assert!(!properties.contains(Properties::LAZILY_ALLOCATED));
            Some(
                (properties.contains(Properties::DEVICE_LOCAL) as u8) << 2 |
                (properties.contains(Properties::COHERENT) as u8) << 1 |
                ((!properties.contains(Properties::CPU_CACHED)) as u8) << 0 |
                0
            )
        }
    }
}

/// CPU to GPU data flow with mapping.
/// Used for staging data before copying to the `DEVICE_LOCAL` memory.
/// Requires `HOST_VISIBLE` memory.
pub struct Upload;

impl Usage for Upload {
    type Key = u8;

    fn key(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::CPU_VISIBLE) {
            None
        } else {
            assert!(!properties.contains(Properties::LAZILY_ALLOCATED));
            Some(
                ((!properties.contains(Properties::DEVICE_LOCAL)) as u8) << 2 |
                ((!properties.contains(Properties::CPU_CACHED)) as u8) << 0 |
                (properties.contains(Properties::COHERENT) as u8) << 1 |
                0
            )
        }
    }
}

/// GPU to CPU data flow with mapping.
/// Used for copying data from `DEVICE_LOCAL` memory to be read by the host.
/// Requires `HOST_VISIBLE` with `HOST_CACHED` preferable.
pub struct Download;

impl Usage for Download {
    type Key = u8;

    fn key(&self, properties: Properties) -> Option<u8> {
        if !properties.contains(Properties::CPU_VISIBLE) {
            None
        } else {
            assert!(!properties.contains(Properties::LAZILY_ALLOCATED));
            Some(
                ((!properties.contains(Properties::DEVICE_LOCAL)) as u8) << 2 |
                (properties.contains(Properties::CPU_CACHED) as u8) << 1 |
                (properties.contains(Properties::COHERENT) as u8) << 0 |
                0
            )
        }
    }
}

/// Dynamic value that specify memory usage flags.
#[derive(Copy, Clone, Debug)]
pub enum Value {
    Data,
    Dynamic,
    Upload,
    Download,
}

impl Usage for Value {
    type Key = u8;

    fn key(&self, properties: Properties) -> Option<u8> {
        match self {
            Value::Data => Data.key(properties),
            Value::Dynamic => Dynamic.key(properties),
            Value::Upload => Upload.key(properties),
            Value::Download => Download.key(properties),
        }
    }
}
