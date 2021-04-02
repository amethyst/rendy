//! TODO: Add virtual resources to the scheduler.
//! These would have absolutely no impact on the generated GPU schedule, but
//! the order command buffers would be generates would be potentially different
//! and have synchronization.

use std::collections::BTreeSet;
use cranelift_entity::{PrimaryMap, entity_impl, ListPool, EntityList};
use cranelift_entity_set::{EntitySetPool, EntitySet, EntitySetIter};

use rendy_core::hal;

pub use crate::interface::EntityId;
use crate::sync::SyncPoint;
use crate::interface::{SemaphoreId, FenceId};
use crate::resources::PartialFormat;

// /// Identifies a single entity uniquely in the scheduler.
//#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
//pub struct EntityId(u32);
//entity_impl!(EntityId, "entity");

/// Identifies a single resource uniquely in the scheduler.
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct ResourceId(u32);
entity_impl!(ResourceId, "resource");

/// Identifies a relationship between an entity and a resource.
/// Only one may exist for every combination of Entity and Resource.
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct ResourceUseId(u32);
entity_impl!(ResourceUseId, "resource_use");

/// Marks the resource as required to be scheduled on the GPU.
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Root {
    Entity(EntityId),
    Resource(ResourceId),
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct RenderPassSpan(EntityId, EntityId);
impl RenderPassSpan {

    pub fn new(one: EntityId, two: EntityId) -> Self {
        if one.0 < two.0 {
            RenderPassSpan(one, two)
        } else {
            RenderPassSpan(two, one)
        }
    }

    pub fn from(&self) -> EntityId {
        self.0
    }

    pub fn to(&self) -> EntityId {
        self.1
    }

}
impl From<(EntityId, EntityId)> for RenderPassSpan {
    fn from(other: (EntityId, EntityId)) -> Self {
        Self::new(other.0, other.1)
    }
}

/// This is the input data for the scheduler.
/// A couple of invariants MUST be upheld for things to function properly:
/// 1. Entities MUST be in a topologically sorted order. With most normal
///    builders this should fall pretty naturally.
/// 2. There must only be one relationship between each entity and resource.
/// 3. Dependency ordering outside of the natural entity ordering is
///    unrepresentable, and therefore can't exist.
pub trait SchedulerInput {
    /// Entries are primitive schedulable pieces of work on the GPU.
    fn num_entities(&self) -> usize;
    /// Resources are pieces of data on the GPU (typically images and buffers),
    /// that the entities to be scheduled interact with.
    fn num_resources(&self) -> usize;
    /// Given a resource id, returns the usage indices for the resource.
    /// The uses MUST occur in the same order as entity ids, that is, entity ids
    /// start low and monotonically increase.
    fn get_uses(&self, resource: ResourceId) -> &[ResourceUseId];
    /// Given a resource use id, returns metadata for that resource use.
    fn resource_use_data(&self, resource_use: ResourceUseId) -> ResourceUseData;
    fn resource_data(&self, resource: ResourceId) -> ResourceData;
    /// Fetches the set of entity pairs the scheduler should put in the same
    /// render pass.
    fn get_render_pass_spans(&self, out: &mut Vec<RenderPassSpan>);
    /// An entity can be either a pass, transfer operation, or a standalone
    /// entity.
    fn entity_kind(&self, entity: EntityId) -> EntityKind;

    // Sync
    fn num_semaphores(&self) -> usize;
    fn get_semaphore(&self, semaphore: SemaphoreId) -> SyncPoint;
    fn has_aquire_semaphore(&self, resource: ResourceId) -> Option<()>;

    fn num_fences(&self) -> usize;
    fn get_fence(&self, semaphore: FenceId) -> SyncPoint;

    fn get_sync_point(&self, sync_point: SyncPoint) -> SyncPointKind;
}

pub struct ImageData {
    /// How to handle the resource on first use.
    pub load_op: hal::pass::AttachmentLoadOp,
    /// Whether the image is used after the graph is done with it.
    /// If this is false, the store op of the last use may be don't care.
    pub used_after: bool,
    pub kind: Option<hal::image::Kind>,
    pub format: hal::format::Format,
    pub usage: (hal::image::Access, hal::image::Layout),
}

pub struct BufferData {}

pub enum ResourceData {
    Image(ImageData),
    Buffer(BufferData),
}
impl ResourceData {
    pub fn image(self) -> ImageData {
        match self {
            ResourceData::Image(image) => image,
            _ => panic!(),
        }
    }
}

/// Defines how the entity should be scheduled.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EntityKind {
    /// An entity that needs to be scheduled within a VK subpass.
    Pass,
    /// Transfer type. This can hint to the scheduler that it should be
    /// scheduled as soon as possible after it's dependencies.
    Transfer,
    /// Catchall kind that can contain pretty much anything that can be
    /// scheduled at the top level.
    Standalone,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UseKind {
    /// Any generic usage.
    Use,
    /// Usage as a framebuffer attachment.
    Attachment(hal::image::Layout),
}

#[derive(Copy, Clone)]
pub struct ResourceUseData {
    pub entity: EntityId,
    pub resource: ResourceId,

    /// If the entity writes to the resource.
    /// If this is false, the scheduler may perform reorders.
    pub is_write: bool,

    /// How this resource is used by the entity.
    /// This can either be `Use` for any generic usage, or `Attachment` for
    /// usage as a framebuffer attachment.
    pub use_kind: UseKind,

    pub stages: hal::pso::PipelineStage,

    pub specific_use_data: SpecificResourceUseData,
}

#[derive(Copy, Clone)]
pub enum SpecificResourceUseData {
    Buffer {
        state: hal::buffer::State,
    },
    Image {
        state: hal::image::State,
    },
}
impl SpecificResourceUseData {
    pub fn image_state(&self) -> hal::image::State {
        match self {
            SpecificResourceUseData::Image { state } => *state,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SyncPointKind {
    And(SyncPoint, SyncPoint),
    Resource(ResourceId, usize),
}
