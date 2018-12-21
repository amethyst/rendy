use std::cmp::min;

use crate::memory::{LinearConfig, DynamicConfig, HeapsConfig};

/// Factory initialization config.
#[derive(Clone, derivative::Derivative)]
#[derivative(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config<H = BasicHeapsConfigure, Q = OneGraphicsQueue> {
    /// Config for memory::Heaps.
    pub heaps: H,

    /// Config for queue families.
    pub queues: Q,
}

/// Queues configuration.
pub unsafe trait QueuesConfigure {
    /// Slice of priorities.
    type Priorities: AsRef<[f32]>;

    /// Iterator over families to create.
    type Families: IntoIterator<Item = (gfx_hal::queue::QueueFamilyId, Self::Priorities)>;

    /// Configure.
    fn configure(self, families: &[impl gfx_hal::queue::QueueFamily]) -> Self::Families;
}

/// QueuePicker that picks first graphics queue family.
/// If possible it checks that queues of the family are capabile of presenting.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OneGraphicsQueue;

unsafe impl QueuesConfigure for OneGraphicsQueue {
    type Priorities = [f32; 1];
    type Families = Option<(gfx_hal::queue::QueueFamilyId, [f32; 1])>;
    fn configure(self, families: &[impl gfx_hal::queue::QueueFamily]) -> Option<(gfx_hal::queue::QueueFamilyId, [f32; 1])> {
        families
            .iter()
            .find(|f| f.supports_graphics() && f.max_queues() > 0)
            .map(|f| (f.id(), [1.0]))
    }
}

/// Saved config for queues.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SavedQueueConfig(Vec<(gfx_hal::queue::QueueFamilyId, Vec<f32>)>);

unsafe impl QueuesConfigure for SavedQueueConfig {
    type Priorities = Vec<f32>;
    type Families = Vec<(gfx_hal::queue::QueueFamilyId, Vec<f32>)>;
    fn configure(self, _: &[impl gfx_hal::queue::QueueFamily]) -> Vec<(gfx_hal::queue::QueueFamilyId, Vec<f32>)> {
        self.0
    }
}

/// Heaps configuration.
pub unsafe trait HeapsConfigure {
    /// Iterator over memory types.
    type Types: IntoIterator<Item = (gfx_hal::memory::Properties, u32, HeapsConfig)>;

    /// Iterator over heaps.
    type Heaps: IntoIterator<Item = u64>;

    /// Configure.
    fn configure(
        self,
        properties: &gfx_hal::adapter::MemoryProperties,
    ) -> (Self::Types, Self::Heaps);
}

/// Basic heaps config.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasicHeapsConfigure;

unsafe impl HeapsConfigure for BasicHeapsConfigure {
    type Types = Vec<(gfx_hal::memory::Properties, u32, HeapsConfig)>;
    type Heaps = Vec<u64>;

    fn configure(
        self,
        properties: &gfx_hal::adapter::MemoryProperties,
    ) -> (Self::Types, Self::Heaps) {
        let types = properties.memory_types.iter()
            .map(|mt| {
                let config = HeapsConfig {
                    linear: if mt
                        .properties
                        .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
                    {
                        Some(LinearConfig {
                            linear_size: min(
                                256 * 1024 * 1024,
                                properties.memory_heaps[mt.heap_index as usize] / 8,
                            ),
                        })
                    } else {
                        None
                    },
                    dynamic: Some(DynamicConfig {
                        max_block_size: min(
                            32 * 1024 * 1024,
                            properties.memory_heaps[mt.heap_index as usize] / 8,
                        ),
                        block_size_granularity: min(
                            256,
                            properties.memory_heaps[mt.heap_index as usize] / 1024,
                        ),
                        blocks_per_chunk: 64,
                    }),
                };

                (mt.properties, mt.heap_index as u32, config)
            }).collect();

        let heaps = properties.memory_heaps.iter()
            .cloned()
            .collect();

        (types, heaps)
    }
}

/// Saved config for heaps.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SavedHeapsConfig {
    types: Vec<(gfx_hal::memory::Properties, u32, HeapsConfig)>,
    heaps: Vec<u64>,
}

unsafe impl HeapsConfigure for SavedHeapsConfig {
    type Types = Vec<(gfx_hal::memory::Properties, u32, HeapsConfig)>;
    type Heaps = Vec<u64>;

    fn configure(
        self,
        _properties: &gfx_hal::adapter::MemoryProperties,
    ) -> (Self::Types, Self::Heaps) {
        (self.types, self.heaps)
    }
}
