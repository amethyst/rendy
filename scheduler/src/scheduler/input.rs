//! TODO: Add virtual resources to the scheduler.
//! These would have absolutely no impact on the generated GPU schedule, but
//! the order command buffers would be generates would be potentially different
//! and have synchronization.

use std::collections::BTreeSet;
use cranelift_entity::{PrimaryMap, entity_impl};
use cranelift_entity_set::{EntitySetPool, EntitySet};

/// Identifies a single entity uniquely in the scheduler.
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Entity(u32);
entity_impl!(Entity, "entity");

/// Identifies a single resource uniquely in the scheduler.
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Resource(u32);
entity_impl!(Resource, "resource");

/// Identifies a relationship between an entity and a resource.
/// Only one may exist for every combination of Entity and Resource.
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct ResourceUse(u32);
entity_impl!(ResourceUse, "resource_use");

/// Marks the resource as required to be scheduled on the GPU.
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Root {
    Entity(Entity),
    Resource(Resource),
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct RenderPassSpan(Entity, Entity);
impl RenderPassSpan {

    pub fn new(one: Entity, two: Entity) -> Self {
        if one.0 < two.0 {
            RenderPassSpan(one, two)
        } else {
            RenderPassSpan(two, one)
        }
    }

    pub fn from(&self) -> Entity {
        self.0
    }

    pub fn to(&self) -> Entity {
        self.1
    }

}
impl From<(Entity, Entity)> for RenderPassSpan {
    fn from(other: (Entity, Entity)) -> Self {
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
pub struct SchedulerInput<E, R> {
    pub entity: PrimaryMap<Entity, EntityData<E>>,
    pub resource: PrimaryMap<Resource, ResourceData<R>>,

    pub resource_use: PrimaryMap<ResourceUse, ResourceUseData>,

    pub render_pass_spans: BTreeSet<RenderPassSpan>,
    pub roots: BTreeSet<Root>,

    pub resource_use_set_pool: EntitySetPool<ResourceUse>,
}

impl<E, R> SchedulerInput<E, R> {

    pub fn new() -> Self {
        SchedulerInput {
            entity: PrimaryMap::new(),
            resource: PrimaryMap::new(),

            resource_use: PrimaryMap::new(),

            render_pass_spans: BTreeSet::new(),
            roots: BTreeSet::new(),

            resource_use_set_pool: EntitySetPool::new(),
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

pub struct EntityData<E> {
    pub kind: EntityKind,
    pub uses: EntitySet<ResourceUse>,

    pub aux: E,
}

pub struct ResourceData<R> {
    pub uses: EntitySet<ResourceUse>,

    pub aux: R,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UseKind {
    /// Any generic usage.
    Use,
    /// Usage as a framebuffer attachment.
    Attachment,
}

pub struct ResourceUseData {
    pub entity: Entity,
    pub resource: Resource,

    /// If the entity writes to the resource.
    /// If this is false, the scheduler may perform reorders.
    pub is_write: bool,

    /// How this resource is used by the entity.
    /// This can either be `Use` for any generic usage, or `Attachment` for
    /// usage as a framebuffer attachment.
    pub use_kind: UseKind,
}

