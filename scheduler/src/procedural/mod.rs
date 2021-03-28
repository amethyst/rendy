//! A simple graph builder with relatively few static guarantees.
//! Might be useful to use directly in some cases, but it is expected there
//! will be other builders built on top of this.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use rendy_core::hal;

use cranelift_entity::{entity_impl, EntityList, EntityRef, ListPool, PrimaryMap, SecondaryMap};
use cranelift_entity_set::{EntitySet, EntitySetPool};

mod builder;
mod input;
mod api;

use crate::{
    interface::{
        BufferToken, EntityConstructionError, EntityCtx, EntityId, FenceId, GraphCtx, ImageToken,
        NodeConstructionError, PassEntityCtx, PersistenceToken, PersistentBuffer, PersistentImage,
        PersistentKind, Root, SemaphoreId, VirtualId,
    },
    resources::{
        BufferInfo, BufferUsage, ImageInfo, ImageUsage, ProvidedBufferUsage, ProvidedImageUsage,
        VirtualUsage,
    },
    sync::{HasSyncPoint, SyncPoint, SyncPointRef},
    BufferId, ImageId, IterEither, SchedulerTypes,
};

use crate::input::{RenderPassSpan, ResourceId, ResourceUseData, ResourceUseId, SyncPointKind};

impl From<ImageId> for ResourceId {
    fn from(id: ImageId) -> ResourceId {
        ResourceId::new(id.index())
    }
}
impl Into<ImageId> for ResourceId {
    fn into(self) -> ImageId {
        ImageId::new(self.index())
    }
}
impl From<BufferId> for ResourceId {
    fn from(id: BufferId) -> ResourceId {
        ResourceId::new(id.index())
    }
}
impl Into<BufferId> for ResourceId {
    fn into(self) -> BufferId {
        BufferId::new(self.index())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RequiredRenderPassId(pub(crate) u32);
entity_impl!(RequiredRenderPassId, "required_render_pass");

pub enum ImageSource<T: SchedulerTypes> {
    Owned,
    Provided {
        image: Option<T::Image>,
        acquire: Option<T::Semaphore>,
        provided_image_usage: Option<ProvidedImageUsage>,
    },
}
impl<T: SchedulerTypes> ImageSource<T> {
    pub fn is_owned(&self) -> bool {
        match self {
            ImageSource::Owned => true,
            _ => false,
        }
    }
}

pub(crate) enum ImageUsageKind {
    InputAttachment(usize),
    DepthAttachment,
    Attachment(usize),
    Use,
}

pub(crate) struct ImageUse {
    pub(crate) usage: ImageUsage,
    pub(crate) kind: ImageUsageKind,
    pub(crate) by: EntityId,
}

pub struct ImageData<T: SchedulerTypes> {
    pub(crate) source: ImageSource<T>,
    pub(crate) info: ImageInfo,
    pub(crate) uses: Vec<ImageUse>,
}
impl<T: SchedulerTypes> ImageData<T> {
    pub fn source(&self) -> &ImageSource<T> {
        &self.source
    }
    pub fn source_mut(&mut self) -> &mut ImageSource<T> {
        &mut self.source
    }
    pub fn info(&self) -> &ImageInfo {
        &self.info
    }
    fn last_use(&self) -> Option<EntityId> {
        self.uses.last().map(|u| u.by)
    }
    fn iter_uses<'a>(&'a self) -> impl Iterator<Item = EntityId> + 'a {
        self.uses.iter().map(|u| u.by)
    }
}

pub struct BufferData<T: SchedulerTypes> {
    pub(crate) source: BufferSource<T>,
    pub(crate) info: BufferInfo,
    pub(crate) uses: Vec<BufferUse>,
}
impl<T: SchedulerTypes> BufferData<T> {
    fn last_use(&self) -> Option<EntityId> {
        self.uses.last().map(|u| u.by)
    }
    fn iter_uses<'a>(&'a self) -> impl Iterator<Item = EntityId> + 'a {
        self.uses.iter().map(|u| u.by)
    }
}

pub(crate) enum BufferSource<T: SchedulerTypes> {
    Owned,
    Provided {
        buffer: T::Buffer,
        acquire: Option<T::Semaphore>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    },
}
impl<T: SchedulerTypes> BufferSource<T> {
    pub fn is_owned(&self) -> bool {
        match self {
            BufferSource::Owned => true,
            _ => false,
        }
    }
}

pub(crate) struct BufferUse {
    pub(crate) usage: BufferUsage,
    pub(crate) by: EntityId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum BuildKind {
    None,
    Pass,
    Standalone,
}
impl BuildKind {
    fn none(self) {
        assert!(self == BuildKind::None);
    }
    fn entity(self) {
        assert!(self != BuildKind::None);
    }
    fn pass(self) {
        assert!(self == BuildKind::Pass);
    }
    fn standalone(self) {
        assert!(self == BuildKind::Standalone);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum EntityKind {
    /// An entity that needs to be scheduled within a VK subpass.
    Pass,
    /// Dispatch type. This can hint to the scheduler that it should be
    /// scheduled as soon as possible after it's dependencies.
    Transfer,
    /// Catchall kind that can contain pretty much anything that can be
    /// scheduled at the top level.
    Standalone,
}

pub(crate) struct Entity<T: SchedulerTypes> {
    pub(crate) kind: EntityKind,
    pub(crate) attachments: Option<Attachments>,
    pub(crate) node_data: Option<T::NodeValue>,
}

pub struct Attachments {
    pub depth: Option<ImageId>,
    pub color: Vec<ImageId>,
    pub input: Vec<ImageId>,
}

pub(crate) struct VirtualData {
    uses: Vec<(EntityId, VirtualUsage)>,
}

pub struct Resource<T: SchedulerTypes> {
    pub kind: ResourceKind<T>,
    is_dead: bool,
    processed_uses: EntityList<ResourceUseId>,
}
impl<T: SchedulerTypes> Resource<T> {
    pub fn uses<'a>(&'a self, proc: &'a ProceduralBuilder<T>) -> &'a [ResourceUseId] {
        self.processed_uses.as_slice(&proc.resource_use_list_pool)
    }
}

pub enum ResourceKind<T: SchedulerTypes> {
    Alias(ResourceId),
    Image(ImageData<T>),
    Buffer(BufferData<T>),
}
impl<T: SchedulerTypes> ResourceKind<T> {
    pub fn alias(&self) -> Option<ResourceId> {
        match self {
            ResourceKind::Alias(res) => Some(*res),
            _ => None,
        }
    }

    pub fn unwrap_image(self) -> Option<ImageData<T>> {
        match self {
            ResourceKind::Image(image) => Some(image),
            _ => None,
        }
    }

    pub fn image_ref(&self) -> Option<&ImageData<T>> {
        match self {
            ResourceKind::Image(image) => Some(image),
            _ => None,
        }
    }

    pub fn image_mut(&mut self) -> Option<&mut ImageData<T>> {
        match self {
            ResourceKind::Image(image) => Some(image),
            _ => None,
        }
    }

    pub fn unwrap_buffer(self) -> Option<BufferData<T>> {
        match self {
            ResourceKind::Buffer(buffer) => Some(buffer),
            _ => None,
        }
    }

    pub fn buffer_ref(&self) -> Option<&BufferData<T>> {
        match self {
            ResourceKind::Buffer(buffer) => Some(buffer),
            _ => None,
        }
    }

    pub fn buffer_mut(&mut self) -> Option<&mut BufferData<T>> {
        match self {
            ResourceKind::Buffer(buffer) => Some(buffer),
            _ => None,
        }
    }

    pub fn iter_uses<'a>(&'a self) -> impl Iterator<Item = EntityId> + 'a {
        match self {
            ResourceKind::Buffer(buf) => IterEither::A(buf.iter_uses()),
            ResourceKind::Image(img) => IterEither::B(img.iter_uses()),
            _ => panic!(),
        }
    }
}
impl<T: SchedulerTypes> From<ImageData<T>> for ResourceKind<T> {
    fn from(d: ImageData<T>) -> Self {
        ResourceKind::Image(d)
    }
}
impl<T: SchedulerTypes> From<BufferData<T>> for ResourceKind<T> {
    fn from(d: BufferData<T>) -> Self {
        ResourceKind::Buffer(d)
    }
}

pub struct ProceduralBuilder<T: SchedulerTypes> {
    pub resources: PrimaryMap<ResourceId, Resource<T>>,
    //pub(crate) virtuals: PrimaryMap<VirtualId, VirtualData>,
    entities: PrimaryMap<EntityId, Entity<T>>,

    resource_uses: PrimaryMap<ResourceUseId, ResourceUseData>,
    resource_use_list_pool: ListPool<ResourceUseId>,

    sync_points: PrimaryMap<SyncPoint, SyncPointKind>,
    exported_semaphores: PrimaryMap<SemaphoreId, SyncPoint>,
    exported_fences: PrimaryMap<FenceId, SyncPoint>,
    roots: BTreeSet<Root>,

    render_pass_spans: BTreeSet<RenderPassSpan>,

    // Currently being built
    build_status: BuildKind,
    curr_entity: Option<Entity<T>>,
}

impl<T: SchedulerTypes> ProceduralBuilder<T> {
    pub fn new() -> Self {
        Self {
            resources: PrimaryMap::new(),
            //virtuals: PrimaryMap::new(),
            /// # Invariants
            /// This list will always be topologically sorted, as a concequence
            /// of how the builder works. That is, entities will always appear
            /// in the list in the same relative order as the usage chain of all
            /// resources.
            entities: PrimaryMap::new(),

            resource_uses: PrimaryMap::new(),
            resource_use_list_pool: ListPool::new(),

            sync_points: PrimaryMap::new(),
            exported_fences: PrimaryMap::new(),
            exported_semaphores: PrimaryMap::new(),
            roots: BTreeSet::new(),

            render_pass_spans: BTreeSet::new(),

            build_status: BuildKind::None,
            curr_entity: None,
        }
    }
}
