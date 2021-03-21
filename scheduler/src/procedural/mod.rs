//! A simple graph builder with relatively few static guarantees.
//! Might be useful to use directly in some cases, but it is expected there
//! will be other builders built on top of this.

use std::any::{Any, TypeId};
use std::collections::{BTreeSet, BTreeMap};
use std::ops::{Index, IndexMut};
use std::marker::PhantomData;
use std::convert::TryInto;

use rendy_core::hal;

use cranelift_entity::{PrimaryMap, SecondaryMap, EntityRef, entity_impl, EntityList, ListPool};
use cranelift_entity_set::{EntitySetPool, EntitySet};

mod input;

use crate::{
    ImageId, BufferId,
    IterEither,
    SchedulerTypes,
    interface::{
        GraphCtx, EntityCtx, PassEntityCtx,
        EntityId, SemaphoreId, FenceId, VirtualId,
        Root,
        ImageToken, BufferToken,
        PersistenceToken, PersistentKind, PersistentImage, PersistentBuffer,
        EntityConstructionError, NodeConstructionError,
    },
    resources::{
        ImageInfo, BufferInfo,
        ImageUsage, BufferUsage, VirtualUsage,
        ProvidedImageUsage, ProvidedBufferUsage,
    },
    sync::{SyncPoint, HasSyncPoint, SyncPointRef},
};

use crate::input::{
    ResourceId, SyncPointKind, RenderPassSpan, ResourceUseId, ResourceUseData,
};

//#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
//pub(crate) struct ResourceId(pub(crate) u32);
//entity_impl!(ResourceId, "resource");

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

pub enum ImageKind<T: SchedulerTypes> {
    Owned {
        info: ImageInfo,
    },
    Provided {
        info: ImageInfo,
        image: T::Image,
        acquire: Option<T::Semaphore>,
        provided_image_usage: Option<ProvidedImageUsage>,
    },
}
impl<T: SchedulerTypes> ImageKind<T> {

    pub fn info(&self) -> &ImageInfo {
        match self {
            ImageKind::Owned { info, .. } => info,
            ImageKind::Provided { info, .. } => info,
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
    pub(crate) kind: ImageKind<T>,
    pub(crate) uses: Vec<ImageUse>,
}
impl<T: SchedulerTypes> ImageData<T> {
    pub fn kind(&self) -> &ImageKind<T> {
        &self.kind
    }
    fn last_use(&self) -> Option<EntityId> {
        self.uses.last().map(|u| u.by)
    }
    fn iter_uses<'a>(&'a self) -> impl Iterator<Item = EntityId> + 'a {
        self.uses.iter().map(|u| u.by)
    }
}

pub struct BufferData<T: SchedulerTypes> {
    pub(crate) kind: BufferKind<T>,
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

pub(crate) enum BufferKind<T: SchedulerTypes> {
    Owned {
        info: BufferInfo
    },
    Provided {
        info: BufferInfo,
        buffer: T::Buffer,
        acquire: Option<T::Semaphore>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    },
}
impl<T: SchedulerTypes> BufferKind<T> {

    pub(crate) fn info(&self) -> &BufferInfo {
        match self {
            BufferKind::Owned { info, .. } => info,
            BufferKind::Provided { info, .. } => info,
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

    pub(crate) images: Vec<ImageToken>,
    pub(crate) buffers: Vec<BufferToken>,

    pub(crate) attachments: Option<Attachments>,

    pub(crate) node_data: Option<T::NodeValue>,
}

pub(crate) struct Attachments {
    pub(crate) depth: Option<ImageToken>,
    pub(crate) color: Vec<ImageToken>,
    pub(crate) input: Vec<ImageToken>,
}


pub(crate) struct VirtualData {
    uses: Vec<(EntityId, VirtualUsage)>,
}

//pub(crate) enum MaybeAlias<I, T> {
//    Alias(I),
//    Inner(T),
//}
//impl<I, T> MaybeAlias<I, T> {
//    pub fn is_alias(&self) -> bool {
//        match self {
//            MaybeAlias::Alias(_) => true,
//            MaybeAlias::Inner(_) => false,
//        }
//    }
//    pub fn inner(&self) -> &T {
//        match self {
//            MaybeAlias::Alias(_) => panic!("attempted to access inner on alias"),
//            MaybeAlias::Inner(inner) => inner,
//        }
//    }
//    pub fn inner_mut(&mut self) -> &mut T {
//        match self {
//            MaybeAlias::Alias(_) => panic!("attempted to access inner on alias"),
//            MaybeAlias::Inner(inner) => inner,
//        }
//    }
//    pub fn unwrap(self) -> T {
//        match self {
//            MaybeAlias::Alias(_) => panic!("attempted to access inner on alias"),
//            MaybeAlias::Inner(inner) => inner,
//        }
//    }
//}

pub struct Resource<T: SchedulerTypes> {
    pub kind: ResourceKind<T>,
    processed_uses: EntityList<ResourceUseId>,
}

pub enum ResourceKind<T: SchedulerTypes> {
    Alias(ResourceId),
    Image(ImageData<T>),
    Buffer(BufferData<T>),
}
impl<T: SchedulerTypes> ResourceKind<T> {

    pub fn is_alias(&self) -> Option<ResourceId> {
        match self {
            ResourceKind::Alias(res) => Some(*res),
            _ => None,
        }
    }

    pub fn is_image(&self) -> bool {
        match self {
            ResourceKind::Image(_) => true,
            _ => false,
        }
    }

    pub fn is_buffer(&self) -> bool {
        match self {
            ResourceKind::Buffer(_) => true,
            _ => false,
        }
    }

    pub fn image(&self) -> &ImageData<T> {
        match self {
            ResourceKind::Image(img) => img,
            _ => panic!(),
        }
    }
    pub fn image_mut(&mut self) -> &mut ImageData<T> {
        match self {
            ResourceKind::Image(img) => img,
            _ => panic!(),
        }
    }
    pub fn unwrap_image(self) -> ImageData<T> {
        match self {
            ResourceKind::Image(img) => img,
            _ => panic!(),
        }
    }

    pub fn buffer(&self) -> &BufferData<T> {
        match self {
            ResourceKind::Buffer(buf) => buf,
            _ => panic!(),
        }
    }
    pub fn buffer_mut(&mut self) -> &mut BufferData<T> {
        match self {
            ResourceKind::Buffer(buf) => buf,
            _ => panic!(),
        }
    }
    pub fn unwrap_buffer(self) -> BufferData<T> {
        match self {
            ResourceKind::Buffer(buf) => buf,
            _ => panic!(),
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

    /// Starts building a pass entity
    pub fn start_pass(&mut self) {
        self.build_status.none();
        self.build_status = BuildKind::Pass;
        self.curr_entity = Some(Entity {
            kind: EntityKind::Pass,
            images: Vec::new(),
            buffers: Vec::new(),
            attachments: Some(Attachments {
                depth: None,
                color: Vec::new(),
                input: Vec::new(),
            }),
            node_data: None,
        });
    }

    /// Starts building a standalone entity
    pub fn start_standalone(&mut self) {
        self.build_status.none();
        self.build_status = BuildKind::Standalone;
        self.curr_entity = Some(Entity {
            kind: EntityKind::Standalone,
            images: Vec::new(),
            buffers: Vec::new(),
            attachments: None,
            node_data: None,
        });
    }

    /// Commits the entity being built
    pub fn commit(&mut self, data: T::NodeValue) {
        self.build_status.entity();
        self.build_status = BuildKind::None;

        let mut entity = self.curr_entity.take().unwrap();
        entity.node_data = Some(data);
        self.entities.push(entity);
    }

    pub fn remove_data(&mut self, entity: EntityId) -> Option<T::NodeValue> {
        self.entities[entity].node_data.take()
    }

    /// Marks that the given entities are required to be scheduled into the same
    /// render pass. If, for some reason, they can't, a graph validation error
    /// will be emitted.
    ///
    /// If all of the given entities are not render passes, then this will
    /// guarantee failure.
    ///
    /// Order does not matter, they will be scheduled in dependency order.
    pub fn mark_render_pass(&mut self, a: EntityId, b: EntityId) {
        self.render_pass_spans.insert(RenderPassSpan::new(a, b));
    }

    fn image_is_transient(&mut self, image: ImageId) -> bool {
        self.resources[image.into()].kind.image().kind.info().mode.is_transient()
    }

    fn handle_image_add_dep(&mut self, image: ImageId) {
        let entity = self.id();
        let num_uses = self.resources[image.into()].kind.image().uses.len();

        if num_uses > 0 {
            let last_use = self.resources[image.into()].kind.image().last_use().unwrap();

            // If the resource is transient, we need to add it to the render pass.
            if self.image_is_transient(image) {
                self.mark_render_pass(last_use, entity);
            }

        }

    }

}

impl<T: SchedulerTypes> GraphCtx<T> for ProceduralBuilder<T> {

    fn mark_root<R: Into<Root>>(&mut self, root: R) {
        self.roots.insert(root.into());
    }

    fn create_virtual(&mut self) -> VirtualId {
        //self.virtuals.push(VirtualData {
        //    uses: Vec::new(),
        //})
        unimplemented!()
    }

    fn create_image(&mut self, info: ImageInfo) -> ImageId {
        self.resources.push(Resource {
            kind: ResourceKind::Image(ImageData {
                kind: ImageKind::Owned {
                    info,
                },
                uses: Vec::new(),
            }),
            processed_uses: EntityList::new(),
        }).into()
    }

    fn provide_image(
        &mut self,
        info: ImageInfo,
        image: impl Into<T::Image>,
        acquire: Option<T::Semaphore>,
        provided_image_usage: Option<ProvidedImageUsage>,
    ) -> ImageId {
        self.resources.push(Resource {
            kind: ResourceKind::Image(ImageData {
                kind: ImageKind::Provided {
                    info,
                    image: image.into(),
                    acquire,
                    provided_image_usage,
                },
                uses: Vec::new(),
            }),
            processed_uses: EntityList::new(),
        }).into()
    }

    fn move_image(&mut self, from: ImageId, to: ImageId) {
        assert!(self.resources[from.into()].kind.is_alias().is_none());
        assert!(self.resources[to.into()].kind.is_alias().is_none());
        assert!(self.resources[to.into()].kind.image().uses.len() == 0);

        // TODO validate kind

        let mut from_data = ResourceKind::Alias(to.into());
        std::mem::swap(&mut from_data, &mut self.resources[from.into()].kind);
        let mut from_inner = from_data.unwrap_image();

        let to_inner = self.resources[to.into()].kind.image_mut();

        to_inner.kind = from_inner.kind;
        to_inner.uses = from_inner.uses;
    }

    fn create_buffer(&mut self, info: BufferInfo) -> BufferId {
        self.resources.push(Resource {
            kind: ResourceKind::Buffer(BufferData {
                kind: BufferKind::Owned {
                    info,
                },
                uses: Vec::new(),
            }),
            processed_uses: EntityList::new(),
        }).into()
    }

    fn provide_buffer(
        &mut self,
        info: BufferInfo,
        buffer: impl Into<T::Buffer>,
        acquire: Option<T::Semaphore>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    ) -> BufferId {
        self.resources.push(
            Resource {
                kind: ResourceKind::Buffer(BufferData {
                    kind: BufferKind::Provided {
                        info,
                        buffer: buffer.into(),
                        acquire,
                        provided_buffer_usage,
                    },
                    uses: Vec::new(),
                }),
                processed_uses: EntityList::new(),
            }
        ).into()
    }

    fn move_buffer(&mut self, from: BufferId, to: BufferId) {
        assert!(self.resources[from.into()].kind.is_alias().is_none());
        assert!(self.resources[to.into()].kind.is_alias().is_none());
        assert!(self.resources[to.into()].kind.buffer().uses.len() == 0);

        // TODO validate kind

        let mut from_data = ResourceKind::Alias(to.into());
        std::mem::swap(&mut from_data, &mut self.resources[from.into()].kind);
        let mut from_inner = from_data.unwrap_buffer();

        let to_inner = self.resources[to.into()].kind.buffer_mut();

        to_inner.kind = from_inner.kind;
        to_inner.uses = from_inner.uses;
    }

    fn create_persistence_token<K: PersistentKind>(&mut self) -> PersistenceToken<K> {
        unimplemented!()
    }

    fn dispose_persistence_token<K: PersistentKind>(&mut self, token: PersistenceToken<K>) {
        unimplemented!()
    }

    fn sync_point_get<A: HasSyncPoint>(&mut self, a: A) -> SyncPoint {
        let spk = a.into_sync_point();
        match spk {
            SyncPointRef::SyncPoint(sp) => sp,
            SyncPointRef::Image(image) => {
                //let tok = ImageToken(
                //    image,
                //    self.resources[image.into()].kind.image().uses.len().try_into().unwrap(),
                //);
                self.sync_points.push(SyncPointKind::Resource(image.into()))
            },
            SyncPointRef::Buffer(buffer) => {
                //let tok = BufferToken(
                //    buffer,
                //    self.resources[buffer.into()].kind.buffer().uses.len().try_into().unwrap(),
                //);
                self.sync_points.push(SyncPointKind::Resource(buffer.into()))
            },
        }
    }

    fn sync_point_and<A1: HasSyncPoint, A2: HasSyncPoint>(
        &mut self,
        a: A1,
        b: A2,
    ) -> SyncPoint {
        let a_sp = self.sync_point_get(a);
        let b_sp = self.sync_point_get(b);

        self.sync_points.push(SyncPointKind::And(a_sp, b_sp))
    }

    fn sync_point_to_semaphore<A: HasSyncPoint>(&mut self, dep: A) -> SemaphoreId {
        let sp = self.sync_point_get(dep);
        self.exported_semaphores.push(sp)
    }

    fn sync_point_to_fence<A: HasSyncPoint>(&mut self, dep: A) -> FenceId {
        let sp = self.sync_point_get(dep);
        self.exported_fences.push(sp)
    }

    //fn sync_point_on<A: HasSyncPoint, F>(&mut self, fun: F)
    //where
    //    F: FnOnce()
    //{
    //    unimplemented!()
    //}

}

impl<T: SchedulerTypes> EntityCtx<T> for ProceduralBuilder<T> {

    fn id(&self) -> EntityId {
        self.entities.next_key()
    }

    fn use_virtual(&mut self, id: VirtualId, usage: VirtualUsage) {
        //let entity_id = self.id();
        //self.virtuals[id].uses.push((entity_id, usage));
        unimplemented!()
    }

    fn use_image(&mut self, id: ImageId, usage: ImageUsage) -> Result<ImageToken, EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(id);

        let image = self.resources[id.into()].kind.image_mut();

        assert!(image.last_use() != Some(entity_id));

        let tok_id = ImageToken(id, image.uses.len().try_into().unwrap());
        image.uses.push(ImageUse {
            usage,
            kind: ImageUsageKind::Use,
            by: entity_id,
        });
        self.curr_entity.as_mut().unwrap().images.push(tok_id);
        Ok(tok_id)
    }

    fn use_buffer(&mut self, id: BufferId, usage: BufferUsage) -> Result<BufferToken, EntityConstructionError> {
        let entity_id = self.id();
        let buffer = self.resources[id.into()].kind.buffer_mut();

        assert!(buffer.last_use() != Some(entity_id));

        let tok_id = BufferToken(id, buffer.uses.len().try_into().unwrap());
        buffer.uses.push(BufferUse {
            usage,
            by: entity_id,
        });
        self.curr_entity.as_mut().unwrap().buffers.push(tok_id);
        Ok(tok_id)
    }

}

impl<T: SchedulerTypes> PassEntityCtx<T> for ProceduralBuilder<T> {

    fn use_color(&mut self, index: usize, image_id: ImageId, read_access: bool) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let image = self.resources[image_id.into()].kind.image_mut();

        assert!(image.last_use() != Some(entity_id));

        let tok_id = ImageToken(image_id, image.uses.len().try_into().unwrap());

        let usage;
        if read_access {
            usage = ImageUsage {
                layout: hal::image::Layout::ColorAttachmentOptimal,
                stages: hal::pso::PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                access: hal::image::Access::COLOR_ATTACHMENT_WRITE,
            };
        } else {
            usage = ImageUsage {
                layout: hal::image::Layout::ColorAttachmentOptimal,
                stages: hal::pso::PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                access: hal::image::Access::COLOR_ATTACHMENT_READ | hal::image::Access::COLOR_ATTACHMENT_WRITE,
            };
        }

        image.uses.push(ImageUse {
            usage,
            kind: ImageUsageKind::Attachment(index),
            by: entity_id,
        });

        self.curr_entity.as_mut().unwrap().images.push(tok_id);
        self.curr_entity.as_mut().unwrap().attachments.as_mut().unwrap().color.push(tok_id);

        Ok(())
    }

    fn use_depth(&mut self, image_id: ImageId, write_access: bool) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let image = self.resources[image_id.into()].kind.image_mut();

        assert!(image.last_use() != Some(entity_id));

        let tok_id = ImageToken(image_id, image.uses.len().try_into().unwrap());

        let usage;
        if write_access {
            usage = ImageUsage {
                layout: hal::image::Layout::DepthStencilAttachmentOptimal,
                stages: hal::pso::PipelineStage::EARLY_FRAGMENT_TESTS | hal::pso::PipelineStage::LATE_FRAGMENT_TESTS,
                access: hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ | hal::image::Access::DEPTH_STENCIL_ATTACHMENT_WRITE,
            };
        } else {
            usage = ImageUsage {
                layout: hal::image::Layout::DepthStencilReadOnlyOptimal,
                stages: hal::pso::PipelineStage::EARLY_FRAGMENT_TESTS | hal::pso::PipelineStage::LATE_FRAGMENT_TESTS,
                access: hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ,
            };
        };

        image.uses.push(ImageUse {
            usage,
            kind: ImageUsageKind::DepthAttachment,
            by: entity_id,
        });

        self.curr_entity.as_mut().unwrap().images.push(tok_id);

        let mut ntid = Some(tok_id);
        std::mem::swap(&mut self.curr_entity.as_mut().unwrap().attachments.as_mut().unwrap().depth, &mut ntid);
        assert!(ntid.is_none());

        Ok(())
    }

    fn use_input(&mut self, index: usize, image_id: ImageId) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let image = self.resources[image_id.into()].kind.image_mut();

        assert!(image.last_use() != Some(entity_id));

        let tok_id = ImageToken(image_id, image.uses.len().try_into().unwrap());

        image.uses.push(ImageUse {
            usage: ImageUsage {
                layout: hal::image::Layout::ShaderReadOnlyOptimal,
                stages: hal::pso::PipelineStage::FRAGMENT_SHADER,
                access: hal::image::Access::INPUT_ATTACHMENT_READ,
            },
            kind: ImageUsageKind::InputAttachment(index),
            by: entity_id,
        });

        self.curr_entity.as_mut().unwrap().images.push(tok_id);
        self.curr_entity.as_mut().unwrap().attachments.as_mut().unwrap().input.push(tok_id);

        Ok(())
    }

}
