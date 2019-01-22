use std::cmp::min;

use crate::{
    command::FamilyId,
    memory::{LinearConfig, DynamicConfig, HeapsConfig},
};

/// Factory initialization config.
#[derive(Clone, derivative::Derivative)]
#[derivative(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config<D = BasicDevicesConfigure, H = BasicHeapsConfigure, Q = OneGraphicsQueue> {
    /// Config to choose adapter.
    pub devices: D,

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
    type Families: IntoIterator<Item = (FamilyId, Self::Priorities)>;

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
    type Families = Option<(FamilyId, [f32; 1])>;
    fn configure(self, families: &[impl gfx_hal::queue::QueueFamily]) -> Option<(FamilyId, [f32; 1])> {
        families
            .iter()
            .find(|f| f.supports_graphics() && f.max_queues() > 0)
            .map(|f| (f.id(), [1.0]))
    }
}

/// Saved config for queues.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SavedQueueConfig(Vec<(FamilyId, Vec<f32>)>);

unsafe impl QueuesConfigure for SavedQueueConfig {
    type Priorities = Vec<f32>;
    type Families = Vec<(FamilyId, Vec<f32>)>;
    fn configure(self, _: &[impl gfx_hal::queue::QueueFamily]) -> Vec<(FamilyId, Vec<f32>)> {
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


/// Devices configuration.
pub trait DevicesConfigure {
    /// Pick adapter from the slice.
    /// 
    /// # Panics
    /// 
    /// This function may panic if empty slice is provided.
    /// 
    fn pick<B>(&self, adapters: &[gfx_hal::Adapter<B>]) -> usize
    where
        B: gfx_hal::Backend,
    ;
}

/// Basics adapters config.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasicDevicesConfigure;

impl DevicesConfigure for BasicDevicesConfigure {
    fn pick<B>(&self, adapters: &[gfx_hal::Adapter<B>]) -> usize
    where
        B: gfx_hal::Backend,
    {
        adapters
            .iter()
            .enumerate()
            .min_by_key(|(_, adapter)| match adapter.info.device_type {
                gfx_hal::adapter::DeviceType::DiscreteGpu => 0,
                gfx_hal::adapter::DeviceType::IntegratedGpu => 1,
                gfx_hal::adapter::DeviceType::VirtualGpu => 2,
                gfx_hal::adapter::DeviceType::Cpu => 3,
                _ => 4,
            })
            .expect("No adapters present")
            .0
    }
}
