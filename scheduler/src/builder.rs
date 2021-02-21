//! A simple graph builder with relatively few static guarantees.
//! Might be useful to use directly in some cases, but it is expected there
//! will be other builders built on top of this.

use std::any::{Any, TypeId};
use std::collections::{BTreeSet, BTreeMap};
use std::ops::{Index, IndexMut};
use std::marker::PhantomData;
use std::convert::TryInto;

use rendy_core::hal;

use cranelift_entity::{PrimaryMap, SecondaryMap, EntityRef, entity_impl};
use cranelift_entity_set::{EntitySetPool, EntitySet};

use crate::{
    ImageId, BufferId,
    IterEither,
    interface::{
        GraphCtx, EntityCtx, PassEntityCtx,
        EntityId, SemaphoreId, FenceId, VirtualId,
        Root,
        ImageToken, BufferToken,
        PersistenceToken, PersistentKind, PersistentImage, PersistentBuffer,
        EntityConstructionError, NodeConstructionError,
    },
    scheduler::{
        SchedulerInput, RenderPassSpan,
    },
    resources::{
        ImageInfo, BufferInfo,
        ImageUsage, BufferUsage, VirtualUsage,
        ProvidedImageUsage, ProvidedBufferUsage,
    },
    sync::{SyncPoint, HasSyncPoint, SyncPointRef},
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ResourceId(pub(crate) u32);
entity_impl!(ResourceId, "resource");

impl From<ImageId> for ResourceId {
    fn from(id: ImageId) -> ResourceId {
        ResourceId(id.0.try_into().unwrap())
    }
}
impl Into<ImageId> for ResourceId {
    fn into(self) -> ImageId {
        ImageId(self.0.try_into().unwrap())
    }
}
impl From<BufferId> for ResourceId {
    fn from(id: BufferId) -> ResourceId {
        ResourceId(id.0.try_into().unwrap())
    }
}
impl Into<BufferId> for ResourceId {
    fn into(self) -> BufferId {
        BufferId(self.0.try_into().unwrap())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RequiredRenderPassId(pub(crate) u32);
entity_impl!(RequiredRenderPassId, "required_render_pass");

pub(crate) enum ImageKind {
    Owned {
        info: ImageInfo,
    },
    Provided {
        info: ImageInfo,
        //image: Handle<Image<B>>,
        //acquire: Option<B::Semaphore>,
        provided_image_usage: Option<ProvidedImageUsage>,
    },
}
impl ImageKind {

    pub(crate) fn info(&self) -> &ImageInfo {
        match self {
            ImageKind::Owned { info, .. } => info,
            ImageKind::Provided { info, .. } => info,
        }
    }

}

pub(crate) enum ImageUsageKind {
    InputAttachment,
    Attachment,
    Use,
}

pub(crate) struct ImageUse {
    pub(crate) usage: ImageUsage,
    pub(crate) kind: ImageUsageKind,
    pub(crate) by: EntityId,
}

pub(crate) struct ImageData {
    pub(crate) kind: ImageKind,
    pub(crate) uses: Vec<ImageUse>,
}
impl ImageData {
    fn last_use(&self) -> Option<EntityId> {
        self.uses.last().map(|u| u.by)
    }
    fn iter_uses<'a>(&'a self) -> impl Iterator<Item = EntityId> + 'a {
        self.uses.iter().map(|u| u.by)
    }
}

pub(crate) struct BufferData {
    pub(crate) kind: BufferKind,
    pub(crate) uses: Vec<BufferUse>,
}
impl BufferData {
    fn last_use(&self) -> Option<EntityId> {
        self.uses.last().map(|u| u.by)
    }
    fn iter_uses<'a>(&'a self) -> impl Iterator<Item = EntityId> + 'a {
        self.uses.iter().map(|u| u.by)
    }
}

pub(crate) enum BufferKind {
    Owned {
        info: BufferInfo
    },
    Provided {
        info: BufferInfo,
        //buffer: Handle<Buffer<B>>,
        //acquire: Option<B::Semaphore>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    },
}
impl BufferKind {

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
        assert!(self == BuildKind::Pass);
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

pub(crate) struct Entity {
    pub(crate) kind: EntityKind,

    pub(crate) images: Vec<ImageToken>,
    pub(crate) buffers: Vec<BufferToken>,

    pub(crate) attachments: Option<Attachments>,
}

pub(crate) struct Attachments {
    pub(crate) depth: Option<ImageToken>,
    pub(crate) color: Vec<ImageToken>,
    pub(crate) input: Vec<ImageToken>,
}

pub(crate) enum SyncPointKind {
    And(Vec<SyncPoint>),
    Image(ImageToken),
    Buffer(BufferToken),
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

pub(crate) enum ResourceKind {
    Alias(ResourceId),
    Image(ImageData),
    Buffer(BufferData),
}
impl ResourceKind {

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

    pub fn image(&self) -> &ImageData {
        match self {
            ResourceKind::Image(img) => img,
            _ => panic!(),
        }
    }
    pub fn image_mut(&mut self) -> &mut ImageData {
        match self {
            ResourceKind::Image(img) => img,
            _ => panic!(),
        }
    }
    pub fn unwrap_image(self) -> ImageData {
        match self {
            ResourceKind::Image(img) => img,
            _ => panic!(),
        }
    }

    pub fn buffer(&self) -> &BufferData {
        match self {
            ResourceKind::Buffer(buf) => buf,
            _ => panic!(),
        }
    }
    pub fn buffer_mut(&mut self) -> &mut BufferData {
        match self {
            ResourceKind::Buffer(buf) => buf,
            _ => panic!(),
        }
    }
    pub fn unwrap_buffer(self) -> BufferData {
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
impl From<ImageData> for ResourceKind {
    fn from(d: ImageData) -> Self {
        ResourceKind::Image(d)
    }
}
impl From<BufferData> for ResourceKind {
    fn from(d: BufferData) -> Self {
        ResourceKind::Buffer(d)
    }
}

pub struct ProceduralBuilder {
    pub(crate) resources: PrimaryMap<ResourceId, ResourceKind>,
    //pub(crate) virtuals: PrimaryMap<VirtualId, VirtualData>,

    pub(crate) entities: PrimaryMap<EntityId, Entity>,

    pub(crate) sync_points: PrimaryMap<SyncPoint, SyncPointKind>,
    pub(crate) exported_semaphores: PrimaryMap<SemaphoreId, SyncPoint>,
    pub(crate) exported_fences: PrimaryMap<FenceId, SyncPoint>,
    pub(crate) roots: BTreeSet<Root>,

    pub(crate) render_pass_spans: Vec<(EntityId, EntityId)>,

    pub(crate) entity_set_pool: EntitySetPool<EntityId>,

    // Currently being built
    pub(crate) build_status: BuildKind,
    pub(crate) curr_entity: Option<Entity>,
}

impl ProceduralBuilder {

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

            sync_points: PrimaryMap::new(),
            exported_fences: PrimaryMap::new(),
            exported_semaphores: PrimaryMap::new(),
            roots: BTreeSet::new(),

            render_pass_spans: Vec::new(),

            entity_set_pool: EntitySetPool::new(),

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
        });
    }

    /// Commits the entity being built
    pub fn commit(&mut self) {
        self.build_status.entity();
        self.build_status = BuildKind::None;

        let entity = self.curr_entity.take().unwrap();
        self.entities.push(entity);
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
        self.render_pass_spans.push((a, b));
    }

    fn image_is_transient(&mut self, image: ImageId) -> bool {
        self.resources[image.into()].image().kind.info().mode.is_transient()
    }

    fn handle_image_add_dep(&mut self, image: ImageId) {
        let entity = self.id();
        let num_uses = self.resources[image.into()].image().uses.len();

        if num_uses > 0 {
            let last_use = self.resources[image.into()].image().last_use().unwrap();

            // If the resource is transient, we need to add it to the render pass.
            if self.image_is_transient(image) {
                self.mark_render_pass(last_use, entity);
            }

        }

    }

    // TODO: Temporary
    pub fn make_scheduler_input(&self) -> SchedulerInput<(), ()> {
        use crate::scheduler::input::{
            Entity, EntityData, EntityKind as EntityKindI, ResourceData,
            ResourceUseData, UseKind,
        };

        fn propagate<I: Copy + Eq + Ord>(map: &mut BTreeMap<I, I>) {
            let keys: Vec<_> = map.keys().cloned().collect();
            loop {
                let mut changed = false;

                for key in keys.iter() {
                    let to_1 = map[&key];
                    if let Some(to_2) = map.get(&to_1).cloned() {
                        map.insert(*key, to_2);
                        changed = true;
                    }
                }

                if !changed { break; }
            }
        }

        fn resolve_aliases<I: EntityRef + Ord, T, F>(
            vec: &PrimaryMap<I, T>, resolved: &mut BTreeMap<I, I>, fun: F)
        where
            F: Fn(&T) -> Option<I>,
        {
            debug_assert!(resolved.len() == 0);

            for (id, item) in vec.iter() {
                if let Some(alias) = fun(item) {
                    resolved.insert(id, alias);
                }
            }

            propagate(resolved);
        }

        let mut resolved = BTreeMap::new();
        resolve_aliases(&self.resources, &mut resolved, |data| {
            if let ResourceKind::Alias(to) = data {
                Some(*to)
            } else {
                None
            }
        });

        let mut input = SchedulerInput::<(), ()>::new();

        for (o_idx, entity) in self.entities.iter() {
            let idx = input.entity.push(EntityData {
                kind: match entity.kind {
                    EntityKind::Pass => EntityKindI::Pass,
                    EntityKind::Transfer => EntityKindI::Transfer,
                    EntityKind::Standalone => EntityKindI::Standalone,
                },
                uses: EntitySet::new(),
                aux: (),
            });
            assert!(o_idx.index() == idx.index());
        }

        for (o_idx, resource) in self.resources.iter() {
            let idx = input.resource.push(ResourceData {
                uses: EntitySet::new(),
                aux: (),
            });
            assert!(o_idx.index() == idx.index());

            match resource {
                ResourceKind::Alias(_) => (),
                ResourceKind::Image(data) => {
                    for use_data in data.uses.iter() {
                        let by = Entity::new(use_data.by.index());
                        let (use_kind, is_write) = match use_data.kind {
                            ImageUsageKind::InputAttachment => {
                                assert!(!use_data.usage.is_write());
                                (UseKind::Attachment, false)
                            },
                            ImageUsageKind::Attachment => {
                                (UseKind::Attachment, true)
                            },
                            ImageUsageKind::Use => {
                                (UseKind::Use, use_data.usage.is_write())
                            },
                        };

                        let resource_use = input.resource_use.push(ResourceUseData {
                            entity: by,
                            resource: idx,
                            use_kind,
                            is_write,
                        });
                        let resource = input.resource[idx].uses.insert(
                            resource_use, &mut input.resource_use_set_pool);
                    }
                },
                ResourceKind::Buffer(data) => {
                    for use_data in data.uses.iter() {
                        let by = Entity::new(use_data.by.index());
                        let resource_use = input.resource_use.push(ResourceUseData {
                            entity: by,
                            resource: idx,
                            use_kind: UseKind::Use,
                            is_write: use_data.usage.is_write(),
                        });
                        let resource = input.resource[idx].uses.insert(
                            resource_use, &mut input.resource_use_set_pool);
                    }
                },
            }
        }

        for (a, b) in self.render_pass_spans.iter() {
            let a_n = Entity::new(a.index());
            let b_n = Entity::new(b.index());

            input.render_pass_spans.insert(RenderPassSpan::new(a_n, b_n));
        }

        input
    }

}

impl GraphCtx for ProceduralBuilder {

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
        self.resources.push(ResourceKind::Image(ImageData {
            kind: ImageKind::Owned {
                info,
            },
            uses: Vec::new(),
        })).into()
    }

    fn provide_image(
        &mut self,
        info: ImageInfo,
        //image: Handle<Image<B>>,
        //acquire: Option<B::Semaphore>,
        provided_image_usage: Option<ProvidedImageUsage>,
    ) -> ImageId {
        self.resources.push(ResourceKind::Image(ImageData {
            kind: ImageKind::Provided {
                info,
                //image,
                //acquire,
                provided_image_usage,
            },
            uses: Vec::new(),
        })).into()
    }

    fn move_image(&mut self, from: ImageId, to: ImageId) {
        assert!(self.resources[from.into()].is_alias().is_none());
        assert!(self.resources[to.into()].is_alias().is_none());
        assert!(self.resources[to.into()].image().uses.len() == 0);

        // TODO validate kind

        let mut from_data = ResourceKind::Alias(to.into());
        std::mem::swap(&mut from_data, &mut self.resources[from.into()]);
        let mut from_inner = from_data.unwrap_image();

        let to_inner = self.resources[to.into()].image_mut();

        to_inner.kind = from_inner.kind;
        to_inner.uses = from_inner.uses;
    }

    fn create_buffer(&mut self, info: BufferInfo) -> BufferId {
        self.resources.push(ResourceKind::Buffer(BufferData {
            kind: BufferKind::Owned {
                info,
            },
            uses: Vec::new(),
        })).into()
    }

    fn provide_buffer(
        &mut self,
        info: BufferInfo,
        //buffer: Handle<Buffer<B>>,
        //acquire: Option<B::Semaphore>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    ) -> BufferId {
        self.resources.push(ResourceKind::Buffer(BufferData {
            kind: BufferKind::Provided {
                info,
                //buffer,
                //acquire,
                provided_buffer_usage,
            },
            uses: Vec::new(),
        })).into()
    }

    fn move_buffer(&mut self, from: BufferId, to: BufferId) {
        assert!(self.resources[from.into()].is_alias().is_none());
        assert!(self.resources[to.into()].is_alias().is_none());
        assert!(self.resources[to.into()].buffer().uses.len() == 0);

        // TODO validate kind

        let mut from_data = ResourceKind::Alias(to.into());
        std::mem::swap(&mut from_data, &mut self.resources[from.into()]);
        let mut from_inner = from_data.unwrap_buffer();

        let to_inner = self.resources[to.into()].buffer_mut();

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
                let tok = ImageToken(
                    image,
                    (self.resources[image.into()].image().uses.len() - 1).try_into().unwrap(),
                );
                self.sync_points.push(SyncPointKind::Image(tok))
            },
            SyncPointRef::Buffer(buffer) => {
                let tok = BufferToken(
                    buffer,
                    (self.resources[buffer.into()].buffer().uses.len() - 1).try_into().unwrap(),
                );
                self.sync_points.push(SyncPointKind::Buffer(tok))
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

        self.sync_points.push(SyncPointKind::And(vec![a_sp, b_sp]))
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

impl EntityCtx for ProceduralBuilder {

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

        let image = self.resources[id.into()].image_mut();

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
        let buffer = self.resources[id.into()].buffer_mut();

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

impl PassEntityCtx for ProceduralBuilder {

    fn use_color(&mut self, index: usize, image_id: ImageId, read_access: bool) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let image = self.resources[image_id.into()].image_mut();

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
            kind: ImageUsageKind::Attachment,
            by: entity_id,
        });

        self.curr_entity.as_mut().unwrap().images.push(tok_id);
        self.curr_entity.as_mut().unwrap().attachments.as_mut().unwrap().color.push(tok_id);

        Ok(())
    }

    fn use_depth(&mut self, image_id: ImageId, write_access: bool) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let image = self.resources[image_id.into()].image_mut();

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
            kind: ImageUsageKind::Attachment,
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

        let image = self.resources[image_id.into()].image_mut();

        assert!(image.last_use() != Some(entity_id));

        let tok_id = ImageToken(image_id, image.uses.len().try_into().unwrap());

        image.uses.push(ImageUse {
            usage: ImageUsage {
                layout: hal::image::Layout::ShaderReadOnlyOptimal,
                stages: hal::pso::PipelineStage::FRAGMENT_SHADER,
                access: hal::image::Access::INPUT_ATTACHMENT_READ,
            },
            kind: ImageUsageKind::InputAttachment,
            by: entity_id,
        });

        self.curr_entity.as_mut().unwrap().images.push(tok_id);
        self.curr_entity.as_mut().unwrap().attachments.as_mut().unwrap().input.push(tok_id);

        Ok(())
    }

}
