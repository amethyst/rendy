use std::cmp::min;

use crate::{
    command::FamilyId,
    memory::{DynamicConfig, HeapsConfig, LinearConfig},
    util::DeviceId,
};

/// Factory initialization config.
///
/// `devices` - [`DeviceConfigure`] implementation instance to pick physical device.
/// [`BasicDevicesConfigure`] can be used as sane default.
/// `heaps` - [`HeapsConfigure`] implementation instance to cofigure memory allocators.
/// [`BasicHeapsConfigure`] can be used as sane default.
/// `queues` - [`QueuesConfigure`] implementation to configure device queues creation.
/// [`OneGraphicsQueue`] can be used if only one graphics queue will satisfy requirements.
///
/// [`DeviceConfigure`]: trait.DevicesConfigure.html
/// [`BasicDevicesConfigure`]: struct.BasicDevicesConfigure.html
/// [`HeapsConfigure`]: trait.HeapsConfigure.html
/// [`BasicHeapsConfigure`]: struct.BasicHeapsConfigure.html
/// [`QueuesConfigure`]: trait.QueuesConfigure.html
/// [`OneGraphicsQueue`]: struct.OneGraphicsQueue.html
#[derive(Clone, derivative::Derivative)]
#[derivative(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Config<D = BasicDevicesConfigure, H = BasicHeapsConfigure, Q = OneGraphicsQueue> {
    /// Config to choose adapter.
    pub devices: D,

    /// Config for memory::Heaps.
    pub heaps: H,

    /// Config for queue families.
    pub queues: Q,
}

/// Queues configuration.
///
/// Method [`configure`] receives collection of queue families and
/// returns an iterator over family ids and number of queues.
///
/// [`configure`]: trait.QueuesConfigure.html#tymethod.configure
pub unsafe trait QueuesConfigure {
    /// Slice of priorities.
    type Priorities: AsRef<[f32]>;

    /// Iterator over families to create.
    type Families: IntoIterator<Item = (FamilyId, Self::Priorities)>;

    /// Configure.
    fn configure(
        self,
        device: DeviceId,
        families: &[impl gfx_hal::queue::QueueFamily],
    ) -> Self::Families;
}

/// QueuePicker that picks first graphics queue family.
///
/// TODO: Try to pick family that is capable of presenting
/// This is possible in platform-dependent way for some platforms.
///
/// To pick multiple families with require number of queues
/// a custom [`QueuesConfigure`] implementation can be used instead.
///
/// [`QueuesConfigure`]: trait.QueuesConfigure.html
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OneGraphicsQueue;

unsafe impl QueuesConfigure for OneGraphicsQueue {
    type Priorities = [f32; 1];
    type Families = Option<(FamilyId, [f32; 1])>;
    fn configure(
        self,
        device: DeviceId,
        families: &[impl gfx_hal::queue::QueueFamily],
    ) -> Option<(FamilyId, [f32; 1])> {
        families
            .iter()
            .find(|f| f.supports_graphics() && f.max_queues() > 0)
            .map(|f| {
                (
                    FamilyId {
                        device,
                        index: f.id().0,
                    },
                    [1.0],
                )
            })
    }
}

/// Saved config for queues.
/// This config can be loaded from config files
/// in any format supported by serde ecosystem.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedQueueConfig(Vec<(usize, Vec<f32>)>);

unsafe impl QueuesConfigure for SavedQueueConfig {
    type Priorities = Vec<f32>;
    type Families = Vec<(FamilyId, Vec<f32>)>;
    fn configure(
        self,
        device: DeviceId,
        _: &[impl gfx_hal::queue::QueueFamily],
    ) -> Vec<(FamilyId, Vec<f32>)> {
        // TODO: FamilyId should be stored directly once it become serializable.
        self.0
            .into_iter()
            .map(|(id, vec)| (FamilyId { device, index: id }, vec))
            .collect()
    }
}

/// Heaps configuration.
///
/// Method [`configure`] receives memory properties and
/// emits iterator memory types together with configurations for allocators and
/// iterator over heaps sizes.
///
/// [`configure`]: trait.HeapsConfigure.html#tymethod.configure
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
/// It uses some arbitrary values that can be considered sane default
/// for today (year 2019) hardware and software.
///
/// If default allocators configuration is suboptimal for the particular use case
/// a custom [`HeapsConfigure`] implementation can be used instead.
///
/// [`HeapsConfigure`]: trait.HeapsConfigure.html
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasicHeapsConfigure;

unsafe impl HeapsConfigure for BasicHeapsConfigure {
    type Types = Vec<(gfx_hal::memory::Properties, u32, HeapsConfig)>;
    type Heaps = Vec<u64>;

    fn configure(
        self,
        properties: &gfx_hal::adapter::MemoryProperties,
    ) -> (Self::Types, Self::Heaps) {
        let _16mb = 16 * 1024 * 1024;
        let _256mb = 256 * 1024 * 1024;

        let types = properties
            .memory_types
            .iter()
            .map(|mt| {
                let config = HeapsConfig {
                    linear: if mt
                        .properties
                        .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
                    {
                        Some(LinearConfig {
                            linear_size: min(_256mb, properties.memory_heaps[mt.heap_index] / 8),
                        })
                    } else {
                        None
                    },
                    dynamic: Some(DynamicConfig {
                        max_block_size: min(
                            _16mb,
                            (properties.memory_heaps[mt.heap_index] / 32 - 1).next_power_of_two(),
                        ),
                        block_size_granularity: min(
                            256,
                            (properties.memory_heaps[mt.heap_index] / 1024 - 1).next_power_of_two(),
                        ),
                        blocks_per_chunk: 64,
                        max_chunk_size: min(
                            _256mb,
                            (properties.memory_heaps[mt.heap_index] / 8 - 1).next_power_of_two(),
                        ),
                    }),
                };

                (mt.properties, mt.heap_index as u32, config)
            })
            .collect();

        let heaps = properties.memory_heaps.iter().cloned().collect();

        (types, heaps)
    }
}

/// Saved config for allocators.
/// This config can be loaded from config files
/// in any format supported by serde ecosystem.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
/// Picks physical device to use.
pub trait DevicesConfigure {
    /// Pick adapter from the slice.
    ///
    /// # Panics
    ///
    /// This function may panic if empty slice is provided.
    ///
    fn pick<B>(&self, adapters: &[gfx_hal::Adapter<B>]) -> usize
    where
        B: gfx_hal::Backend;
}

/// Basics adapters config.
///
/// It picks first device with highest priority.
/// From highest - discrete GPU, to lowest - CPU.
///
/// To pick among presented discret GPUs,
/// or to intentionally pick integrated GPU when discrete GPU is available
/// a custom [`DeviceConfigure`] implementationcan be used instead.
///
/// [`DeviceConfigure`]: trait.DevicesConfigure.html
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
