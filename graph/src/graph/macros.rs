use rendy_core::hal;

use crate::graph::{Graph, GfxSchedulerTypes, GraphImage};
use crate::scheduler::{
    interface::{
        EntityConstructionError,
        GraphCtx, EntityCtx, PassEntityCtx,
        Root, ImageToken, BufferToken,
        ImageId, BufferId, EntityId, FenceId, SemaphoreId, VirtualId,
        PersistentKind, PersistenceToken,
    },
    sync::{SyncPoint, HasSyncPoint, SyncPointRef},
    resources::{
        ImageInfo, BufferInfo,
        ImageUsage, BufferUsage,
        VirtualUsage,
        ProvidedImageUsage, ProvidedBufferUsage,
    },
    input::ResourceId,
};

macro_rules! forward_entity_ctx_functions {
    ($($field_name:ident).+) => {
        fn id(&self) -> EntityId {
            self.$($field_name).+.id()
        }
        fn use_virtual(
            &mut self,
            id: VirtualId,
            usage: VirtualUsage,
        ) {
            self.$($field_name).+.use_virtual(id, usage)
        }
        fn use_image(
            &mut self,
            id: ImageId,
            usage: ImageUsage,
        ) -> Result<ImageToken, EntityConstructionError> {
            self.$($field_name).+.use_image(id, usage)
        }
        fn use_buffer(
            &mut self,
            id: BufferId,
            usage: BufferUsage,
        ) -> Result<BufferToken, EntityConstructionError> {
            self.$($field_name).+.use_buffer(id, usage)
        }
    };
}

macro_rules! forward_graph_ctx_functions {
    ($($field_name:ident).+) => {
        #[inline(always)]
        fn mark_root<R: Into<Root>>(&mut self, root: R) {
            self.$($field_name).+.mark_root(root)
        }
        #[inline(always)]
        fn mark_dead(&mut self, resource: impl Into<ResourceId>) {
            self.$($field_name).+.mark_dead(resource)
        }
        #[inline(always)]
        fn create_virtual(&mut self) -> VirtualId {
            self.$($field_name).+.create_virtual()
        }
        #[inline(always)]
        fn create_image(&mut self, info: ImageInfo) -> ImageId {
            self.$($field_name).+.create_image(info)
        }
        #[inline(always)]
        fn provide_image(
            &mut self,
            image_info: ImageInfo,
            image: impl Into<GraphImage<B>>,
            acquire: Option<B::Semaphore>,
            provided_image_usage: Option<ProvidedImageUsage>,

        ) -> ImageId {
            self.$($field_name).+.provide_image(image_info, image, acquire, provided_image_usage)
        }
        #[inline(always)]
        fn move_image(&mut self, from: ImageId, to: ImageId) {
            self.$($field_name).+.move_image(from, to);
        }
        #[inline(always)]
        fn create_buffer(&mut self, info: BufferInfo) -> BufferId {
            self.$($field_name).+.create_buffer(info)
        }
        #[inline(always)]
        fn provide_buffer(
            &mut self,
            buffer_info: BufferInfo,
            buffer: impl Into<B::Buffer>,
            acquire: Option<B::Semaphore>,
            provided_buffer_usage: Option<ProvidedBufferUsage>,
        ) -> BufferId {
            self.$($field_name).+.provide_buffer(buffer_info, buffer, acquire, provided_buffer_usage)
        }
        #[inline(always)]
        fn move_buffer(&mut self, from: BufferId, to: BufferId) {
            self.$($field_name).+.move_buffer(from, to)
        }
        #[inline(always)]
        fn create_persistence_token<K: PersistentKind>(&mut self) -> PersistenceToken<K> {
            self.$($field_name).+.create_persistence_token()
        }
        #[inline(always)]
        fn dispose_persistence_token<K: PersistentKind>(&mut self, token: PersistenceToken<K>) {
            self.$($field_name).+.dispose_persistence_token(token)
        }
        #[inline(always)]
        fn sync_point_get<A: HasSyncPoint>(&mut self, a: A) -> SyncPoint {
            self.$($field_name).+.sync_point_get(a)
        }
        #[inline(always)]
        fn sync_point_and<A1: HasSyncPoint, A2: HasSyncPoint>(&mut self, a: A1, b: A2) -> SyncPoint {
            self.$($field_name).+.sync_point_and(a, b)
        }
        #[inline(always)]
        fn sync_point_to_semaphore<A: HasSyncPoint>(&mut self, dep: A) -> SemaphoreId {
            self.$($field_name).+.sync_point_to_semaphore(dep)
        }
        #[inline(always)]
        fn sync_point_to_fence<A: HasSyncPoint>(&mut self, dep: A) -> FenceId {
            self.$($field_name).+.sync_point_to_fence(dep)
        }
        //fn sync_point_on<A: HasSyncPoint, F>(&mut self, fun: F)
        //where
        //    F: FnOnce(),
        //{
        //    self.$field_name.sync_point_on(fun)
        //}
    };
}

impl<'a, 'b, B: hal::Backend> GraphCtx<GfxSchedulerTypes<B>> for super::context::GraphConstructCtx<'a, 'b, B> {
    forward_graph_ctx_functions!(inner.procedural);
}

impl<'a, 'b, B: hal::Backend> GraphCtx<GfxSchedulerTypes<B>> for super::context::PassConstructCtx<'a, 'b, B> {
    forward_graph_ctx_functions!(inner.procedural);
}
impl<'a, 'b, B: hal::Backend> EntityCtx<GfxSchedulerTypes<B>> for super::context::PassConstructCtx<'a, 'b, B> {
    forward_entity_ctx_functions!(inner.procedural);
}

impl<'a, 'b, B: hal::Backend> GraphCtx<GfxSchedulerTypes<B>> for super::context::StandaloneConstructCtx<'a, 'b, B> {
    forward_graph_ctx_functions!(inner.procedural);
}
impl<'a, 'b, B: hal::Backend> EntityCtx<GfxSchedulerTypes<B>> for super::context::StandaloneConstructCtx<'a, 'b, B> {
    forward_entity_ctx_functions!(inner.procedural);
}

impl<'b, B: hal::Backend> GraphCtx<GfxSchedulerTypes<B>> for Graph<'b, B> {
    forward_graph_ctx_functions!(procedural);
}
