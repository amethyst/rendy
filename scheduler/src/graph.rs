use std::any::Any;
use std::sync::Arc;
use std::marker::PhantomData;
use std::collections::{BTreeMap, BTreeSet};

use rendy_core::hal;
use crate::{
    ImageId, BufferId,
    resource::{Buffer, Image, Handle},
    factory::Factory,
};
use super::{
    Parameter, DynamicParameter,
    node::{
        GraphCtx, EntityCtx, PassEntityCtx,
        EntityId, FenceId, SemaphoreId,
        EntityConstructionError, NodeConstructionError,
        UnsetParameterError,
        ImageToken, BufferToken,
        PersistentKind, PersistenceToken,
    },
    sync::{SyncPoint, HasSyncPoint, SyncPointRef},
    resources::{
        ImageInfo, BufferInfo,
        ImageUsage, BufferUsage,
        ProvidedImageUsage, ProvidedBufferUsage,
    },
};

macro_rules! forward_graph_ctx_functions {
    ($field_name:ident) => {
        fn factory(&self) -> &'run Factory<B> {
            self.$field_name.factory()
        }
        fn get_parameter<P: Any>(&self, id: Parameter<P>) -> Result<&P, UnsetParameterError> {
            self.$field_name.get_parameter(id)
        }
        fn put_parameter<P: Any>(&mut self, id: Parameter<P>, param: P) {
            self.$field_name.put_parameter(id, param)
        }
        fn create_image(&mut self, info: ImageInfo) -> ImageId {
            self.$field_name.create_image(info)
        }
        fn provide_image(
            &mut self, image_info: ImageInfo, image: Handle<Image<B>>,
            acquire: Option<SyncPoint>, provided_image_usage: Option<ProvidedImageUsage>,
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
            &mut self, buffer_info: BufferInfo, buffer: Handle<Buffer<B>>,
            acquire: Option<SyncPoint>, provided_buffer_usage: Option<ProvidedBufferUsage>,
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
        fn sync_point_and<A1: HasSyncPoint, A2: HasSyncPoint>(&mut self, a: A1, b: A2) -> SyncPoint {
            self.$field_name.sync_point_and(a, b)
        }
        fn sync_point_root<A: HasSyncPoint>(&mut self, dep: A) {
            self.$field_name.sync_point_root(dep)
        }
        unsafe fn sync_point_from_semaphore(&mut self, sem: B::Semaphore) -> SyncPoint {
            self.$field_name.sync_point_from_semaphore(sem)
        }
        fn sync_point_to_semaphore<A: HasSyncPoint>(&mut self, dep: A) -> (SemaphoreId, hal::pso::PipelineStage) {
            self.$field_name.sync_point_to_semaphore(dep)
        }
        fn sync_point_to_fence<A: HasSyncPoint>(&mut self, dep: A) -> FenceId {
            self.$field_name.sync_point_to_fence(dep)
        }
    };
}

#[derive(Default)]
pub struct ParameterStore {
    params: BTreeMap<DynamicParameter, Box<dyn Any>>,
}
impl ParameterStore {

    pub fn clear(&mut self) {
        self.params.clear();
    }

    pub fn get<T: Any + 'static>(&self, param: Parameter<T>) -> Option<&T> {
        self.params.get(&param.into()).map(|b| b.downcast_ref::<T>().unwrap())
    }

    pub fn put<T: Any + 'static>(&mut self, param: Parameter<T>, value: T) -> bool {
        self.params.insert(param.into(), Box::new(value)).is_some()
    }

}

pub struct GraphData<'graph, B: hal::Backend, T: ?Sized> {
    factory: &'graph Factory<B>,
    _u: PhantomData<(B, T)>,
}

pub struct GraphRun<'graph, 'run, B: hal::Backend, T: ?Sized> {
    ctx: &'run mut GraphData<'graph, B, T>,

    store: ParameterStore,

    //aux: &'run T,
    //factory: &'run Factory<B>,

    images: Vec<ImageKind<B>>,
    buffers: Vec<BufferKind<B>>,

    _u: PhantomData<(&'run B, &'run T)>,
}

pub struct EntityBuildCtx<'eb, 'graph, 'run, B: hal::Backend, T: ?Sized> {
    inner: &'eb mut GraphRun<'graph, 'run, B, T>,
}

pub type PassEntityBuildCtx<'eb, 'graph, 'run, B: hal::Backend, T: ?Sized> = EntityBuildCtx<'eb, 'graph, 'run, B, T>;
pub type EntityBuildSelector<'eb, 'graph, 'run, B: hal::Backend, T: ?Sized> = EntityBuildCtx<'eb, 'graph, 'run, B, T>;

enum ImageKind<B: hal::Backend> {
    Owned {
        image_info: ImageInfo,
    },
    Provided {
        image_info: ImageInfo,
        image: Handle<Image<B>>,
        acquire: Option<SyncPoint>,
        provided_image_usage: Option<ProvidedImageUsage>,
    },
}

enum BufferKind<B: hal::Backend> {
    Owned {
        buffer_info: BufferInfo
    },
    Provided {
        buffer_info: BufferInfo,
        buffer: Handle<Buffer<B>>,
        acquire: Option<SyncPoint>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    },
}

