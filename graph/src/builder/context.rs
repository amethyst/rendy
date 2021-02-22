use std::marker::PhantomData;
use std::any::Any;

use rendy_core::hal;

use crate::exec::ExecCtx;
use crate::factory::Factory;
use crate::command::Queue;
use crate::scheduler::{
    builder::ProceduralBuilder,
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
};

use super::{GfxSchedulerTypes, GraphImage};

macro_rules! forward_entity_ctx_functions {
    ($field_name:ident) => {
        fn id(&self) -> EntityId {
            self.$field_name.id()
        }
        fn use_virtual(
            &mut self,
            id: VirtualId,
            usage: VirtualUsage,
        ) {
            self.$field_name.use_virtual(id, usage)
        }
        fn use_image(
            &mut self,
            id: ImageId,
            usage: ImageUsage,
        ) -> Result<ImageToken, EntityConstructionError> {
            self.$field_name.use_image(id, usage)
        }
        fn use_buffer(
            &mut self,
            id: BufferId,
            usage: BufferUsage,
        ) -> Result<BufferToken, EntityConstructionError> {
            self.$field_name.use_buffer(id, usage)
        }
    };
}

macro_rules! forward_graph_ctx_functions {
    ($field_name:ident) => {
        fn mark_root<R: Into<Root>>(&mut self, root: R) {
            self.$field_name.mark_root(root)
        }
        fn create_virtual(&mut self) -> VirtualId {
            self.$field_name.create_virtual()
        }
        fn create_image(&mut self, info: ImageInfo) -> ImageId {
            self.$field_name.create_image(info)
        }
        fn provide_image(
            &mut self,
            image_info: ImageInfo,
            image: impl Into<GraphImage<B>>,
            acquire: Option<B::Semaphore>,
            provided_image_usage: Option<ProvidedImageUsage>,
        ) -> ImageId {
            self.$field_name.provide_image(image_info, image, acquire, provided_image_usage)
        }
        fn move_image(&mut self, from: ImageId, to: ImageId) {
            self.$field_name.move_image(from, to);
        }
        fn create_buffer(&mut self, info: BufferInfo) -> BufferId {
            self.$field_name.create_buffer(info)
        }
        fn provide_buffer(
            &mut self,
            buffer_info: BufferInfo,
            buffer: impl Into<B::Buffer>,
            acquire: Option<B::Semaphore>,
            provided_buffer_usage: Option<ProvidedBufferUsage>,
        ) -> BufferId {
            self.$field_name.provide_buffer(buffer_info, buffer, acquire, provided_buffer_usage)
        }
        fn move_buffer(&mut self, from: BufferId, to: BufferId) {
            self.$field_name.move_buffer(from, to)
        }
        fn create_persistence_token<K: PersistentKind>(&mut self) -> PersistenceToken<K> {
            self.$field_name.create_persistence_token()
        }
        fn dispose_persistence_token<K: PersistentKind>(&mut self, token: PersistenceToken<K>) {
            self.$field_name.dispose_persistence_token(token)
        }
        fn sync_point_get<A: HasSyncPoint>(&mut self, a: A) -> SyncPoint {
            self.$field_name.sync_point_get(a)
        }
        fn sync_point_and<A1: HasSyncPoint, A2: HasSyncPoint>(&mut self, a: A1, b: A2) -> SyncPoint {
            self.$field_name.sync_point_and(a, b)
        }
        fn sync_point_to_semaphore<A: HasSyncPoint>(&mut self, dep: A) -> SemaphoreId {
            self.$field_name.sync_point_to_semaphore(dep)
        }
        fn sync_point_to_fence<A: HasSyncPoint>(&mut self, dep: A) -> FenceId {
            self.$field_name.sync_point_to_fence(dep)
        }
        //fn sync_point_on<A: HasSyncPoint, F>(&mut self, fun: F)
        //where
        //    F: FnOnce(),
        //{
        //    self.$field_name.sync_point_on(fun)
        //}
    };
}

pub struct GraphConstructCtx<'a, B: hal::Backend> {
    phantom: PhantomData<B>,
    inner: &'a mut ProceduralBuilder<GfxSchedulerTypes<B>>,
}
impl<'a, B: hal::Backend> GraphConstructCtx<'a, B> {
    pub fn pass<'b>(&'b mut self) -> PassConstructCtx<'b, B> {
        self.inner.start_pass();
        PassConstructCtx {
            phantom: PhantomData,
            inner: self.inner,
            relevant: relevant::Relevant,
        }
    }
    pub fn standalone<'b>(&'b mut self) -> StandaloneConstructCtx<'b, B> {
        self.inner.start_standalone();
        StandaloneConstructCtx {
            phantom: PhantomData,
            inner: self.inner,
            relevant: relevant::Relevant,
        }
    }
}
impl<'a, B: hal::Backend> GraphCtx<GfxSchedulerTypes<B>> for GraphConstructCtx<'a, B> {
    forward_graph_ctx_functions!(inner);
}

pub struct PassConstructCtx<'a, B: hal::Backend> {
    phantom: PhantomData<B>,
    inner: &'a mut ProceduralBuilder<GfxSchedulerTypes<B>>,
    relevant: relevant::Relevant,
}
impl<'a, B: hal::Backend> PassConstructCtx<'a, B> {
    pub fn commit<F: FnOnce()>(self, _exec: F) {
        self.inner.commit();
        self.relevant.dispose();
    }
}
impl<'a, B: hal::Backend> GraphCtx<GfxSchedulerTypes<B>> for PassConstructCtx<'a, B> {
    forward_graph_ctx_functions!(inner);
}
impl<'a, B: hal::Backend> EntityCtx<GfxSchedulerTypes<B>> for PassConstructCtx<'a, B> {
    forward_entity_ctx_functions!(inner);
}
impl<'a, B: hal::Backend> PassEntityCtx<GfxSchedulerTypes<B>> for PassConstructCtx<'a, B> {
    fn use_color(
        &mut self,
        index: usize,
        image: ImageId,
        read_access: bool,
    ) -> Result<(), EntityConstructionError> {
        self.inner.use_color(index, image, read_access)
    }
    fn use_depth(
        &mut self,
        image: ImageId,
        write_access: bool,
    ) -> Result<(), EntityConstructionError> {
        self.inner.use_depth(image, write_access)
    }
    fn use_input(
        &mut self,
        index: usize,
        image: ImageId,
    ) -> Result<(), EntityConstructionError> {
        self.inner.use_input(index, image)
    }
}

pub struct StandaloneConstructCtx<'a, B: hal::Backend> {
    phantom: PhantomData<B>,
    inner: &'a mut ProceduralBuilder<GfxSchedulerTypes<B>>,
    relevant: relevant::Relevant,
}
impl<'a, B: hal::Backend> StandaloneConstructCtx<'a, B> {
    pub fn commit<F>(self, _exec: F)
    where
        F: FnOnce(&mut dyn Any, &mut Factory<B>, &mut ExecCtx<B>, &mut Queue<B>),
    {
        self.inner.commit();
        self.relevant.dispose();
    }
}
impl<'a, B: hal::Backend> GraphCtx<GfxSchedulerTypes<B>> for StandaloneConstructCtx<'a, B> {
    forward_graph_ctx_functions!(inner);
}
impl<'a, B: hal::Backend> EntityCtx<GfxSchedulerTypes<B>> for StandaloneConstructCtx<'a, B> {
    forward_entity_ctx_functions!(inner);
}
