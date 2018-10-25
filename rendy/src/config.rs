
use std::cmp::min;

use ash::vk::{
    QueueFlags,
    QueueFamilyProperties,
    MemoryPropertyFlags,
    PhysicalDeviceMemoryProperties,
};
use winit::Window;

use command::FamilyId;
use memory::{allocator, HeapsConfig};

#[derive(Clone, Derivative)]
#[derivative(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config<H = BasicHeapsConfigure, Q = OneGraphicsQueue> {
    /// Application name.
    #[derivative(Default(value = "From::from(\"Rendy\")"))]
    pub app_name: String,

    /// Application version.
    #[derivative(Default(value = "vk_make_version!(0,1,0)"))]
    // #[derivative(Debug(format_with = "fmt_version"))]
    pub app_version: u32,

    /// Config for memory::Heaps.
    pub heaps: H,

    /// Config for queue families.
    pub queues: Q,
}
/// Trait that represents some method to select a queue family.
pub unsafe trait QueuesConfigure {
    type Priorities: AsRef<[f32]>;
    type Families: IntoIterator<Item = (FamilyId, Self::Priorities)>;

    fn configure(self, families: &[QueueFamilyProperties]) -> Self::Families;
}

/// QueuePicket that picks first graphics queue family.
/// If possible it checks that queues of the family are capabile of presenting.

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OneGraphicsQueue;

unsafe impl QueuesConfigure for OneGraphicsQueue {
    type Priorities = [f32; 1];
    type Families = Option<(FamilyId, [f32; 1])>;
    fn configure(self, families: &[QueueFamilyProperties]) -> Option<(FamilyId, [f32; 1])> {
        families
            .iter()
            .position(|f| f.queue_flags.intersects(QueueFlags::GRAPHICS) && f.queue_count > 0)
            .map(|p| (FamilyId(p as u32), [1.0]))
    }
}

/// Saved config for queues.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SavedQueueConfig(Vec<(FamilyId, Vec<f32>)>);

unsafe impl QueuesConfigure for SavedQueueConfig {
    type Priorities = Vec<f32>;
    type Families = Vec<(FamilyId, Vec<f32>)>;
    fn configure(self, families: &[QueueFamilyProperties]) -> Vec<(FamilyId, Vec<f32>)> {
        if !self.0.iter().all(|&(index, ref priorities)| families.get(index.0 as usize).map_or(false, |p| p.queue_count as usize >= priorities.len())) {
            panic!("Config is out of date");
        } else {
            self.0
        }
    }
}

pub unsafe trait HeapsConfigure {
    type Types: IntoIterator<Item = (MemoryPropertyFlags, u32, HeapsConfig)>;
    type Heaps: IntoIterator<Item = u64>;

    fn configure(self, properties: &PhysicalDeviceMemoryProperties) -> (Self::Types, Self::Heaps);
}

/// Basic heaps config.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasicHeapsConfigure;

unsafe impl HeapsConfigure for BasicHeapsConfigure {
    type Types = Vec<(MemoryPropertyFlags, u32, HeapsConfig)>;
    type Heaps = Vec<u64>;

    fn configure(self, properties: &PhysicalDeviceMemoryProperties) -> (Self::Types, Self::Heaps) {
        let types = (0 .. properties.memory_type_count)
            .map(|index| &properties.memory_types[index as usize])
            .map(|mt| {
                let config = HeapsConfig {
                    arena: if mt.property_flags.subset(allocator::ArenaAllocator::properties_required()) {
                        Some(allocator::ArenaConfig {
                            arena_size: min(256 * 1024 * 1024, properties.memory_heaps[mt.heap_index as usize].size / 8),
                        })
                    } else {
                        None
                    },
                    dynamic: if mt.property_flags.subset(allocator::DynamicAllocator::properties_required()) {
                        Some(allocator::DynamicConfig {
                            max_block_size: min(32 * 1024 * 1024, properties.memory_heaps[mt.heap_index as usize].size / 8),
                            block_size_granularity: min(256, properties.memory_heaps[mt.heap_index as usize].size / 1024),
                            blocks_per_chunk: 64,
                        })
                    } else {
                        None
                    },
                };

                (mt.property_flags, mt.heap_index, config)
            })
            .collect();

        let heaps = (0 .. properties.memory_heap_count)
            .map(|index| &properties.memory_heaps[index as usize])
            .map(|heap| heap.size)
            .collect();

        (types, heaps)
    }
}

/// Saved config for heaps.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SavedHeapsConfig {
    types: Vec<(MemoryPropertyFlags, u32, HeapsConfig)>,
    heaps: Vec<u64>
}

unsafe impl HeapsConfigure for SavedHeapsConfig {
    type Types = Vec<(MemoryPropertyFlags, u32, HeapsConfig)>;
    type Heaps = Vec<u64>;

    fn configure(self, _properties: &PhysicalDeviceMemoryProperties) -> (Self::Types, Self::Heaps) {
        (self.types, self.heaps)
    }
}

#[allow(unused)]
fn fmt_version(version: &u32, fmt: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
    write!(fmt, "{}.{}.{}", vk_version_major!(*version), vk_version_minor!(*version), vk_version_patch!(*version))
}
