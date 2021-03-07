//! Terminology:
//! * Node - An entry in the built render graph. Can crate and get Parameters.
//!   Nodes are ordered and constructed in order of dependency of parameters.
//! * Entity - When nodes are constructed, they make any number of entities.
//!   These entities are what are actually scheduled. Examples of entities are
//!   render (sub)passes, compute dispatches, transfers.
//!
//! Safety:
//! All safe functions are guarnteed not to cause memory unsafety.
//! Calling safe functions with the wrong arguments MAY cause incorrect
//! synchronization.

use std::any::Any;
use std::marker::PhantomData;

use cranelift_entity::entity_impl;

use rendy_core::hal;

pub use crate::SchedulerTypes;

use crate::{
    //Parameter, DynamicParameter,
    resources::{
        ImageInfo, BufferInfo,
        ImageUsage, BufferUsage, VirtualUsage,
        ProvidedImageUsage, ProvidedBufferUsage,
    },
    sync::{SyncPoint, HasSyncPoint},
};

pub trait PersistentKind {}

pub enum PersistentImage {}
impl PersistentKind for PersistentImage {}

pub enum PersistentBuffer {}
impl PersistentKind for PersistentBuffer {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersistenceToken<K: PersistentKind>(PhantomData<K>);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageToken(pub(crate) ImageId, pub(crate) u32);
impl ImageToken {
    pub fn image_id(self) -> ImageId {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferToken(pub(crate) BufferId, pub(crate) u32);
impl BufferToken {
    pub fn buffer_id(self) -> BufferId {
        self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub(crate) u32);
entity_impl!(EntityId, "entity");

/// Id of the buffer in graph.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(u32);
entity_impl!(BufferId, "buffer");

/// Id of the image (or target) in graph.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageId(u32);
entity_impl!(ImageId, "image");

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FenceId(pub(crate) u32);
entity_impl!(FenceId, "fence");

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemaphoreId(pub(crate) u32);
entity_impl!(SemaphoreId, "semaphore");

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualId(pub(crate) u32);
entity_impl!(VirtualId, "virtual");

#[derive(Debug)]
pub enum EntityConstructionError {}
#[derive(Debug)]
pub enum NodeConstructionError {}

//pub struct UnsetParameterError {
//    pub parameter: DynamicParameter,
//}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Root {
    Entity(EntityId),
    Image(ImageId),
    Buffer(BufferId),
}
impl Into<Root> for EntityId {
    fn into(self) -> Root {
        Root::Entity(self)
    }
}
impl Into<Root> for ImageId {
    fn into(self) -> Root {
        Root::Image(self)
    }
}
impl Into<Root> for BufferId {
    fn into(self) -> Root {
        Root::Buffer(self)
    }
}

pub trait PassEntityCtx<T: SchedulerTypes>: EntityCtx<T> {

    /// Declare usage of a color attachment in render pass node.
    ///
    /// This is mutually exclusive with `use_image` calls on the same image,
    /// and will a cause construction error when this node is not a `pass`.
    ///
    /// If the `usage` involves a write, any subsequent usages will be dependent
    /// on this modification.
    ///
    /// One should be careful of this causing nodes to be scheduled in a different
    /// order than intended. `Parameter<()>` dependencies between nodes can be used
    /// to signal this order, or you can choose to rely on the defined order of
    /// the graph builder you are using.
    fn use_color(
        &mut self,
        index: usize,
        image: ImageId,
        read_access: bool,
    ) -> Result<(), EntityConstructionError>;

    /// Declare usage of a depth/stencil attachment in render pass node.
    ///
    /// This is mutually exclusive with `use_image` calls on the same image,
    /// and will a cause construction error when this node is not a `pass`.
    ///
    /// If the `usage` involves a write, any subsequent usages will be dependent
    /// on this modification.
    ///
    /// One should be careful of this causing nodes to be scheduled in a different
    /// order than intended. `Parameter<()>` dependencies between nodes can be used
    /// to signal this order, or you can choose to rely on the defined order of
    /// the graph builder you are using.
    fn use_depth(
        &mut self,
        image: ImageId,
        write_access: bool,
    ) -> Result<(), EntityConstructionError>;

    /// Declare usage of an input attachment in render pass node.
    /// Input attachment access is always read-only and limited to fragment shaders.
    ///
    /// This is mutually exclusive with `use_image` calls on the same image,
    /// and will a cause construction error when this node is not a `pass`.
    fn use_input(
        &mut self,
        index: usize,
        image: ImageId,
    ) -> Result<(), EntityConstructionError>;

}

pub trait EntityCtx<T: SchedulerTypes>: GraphCtx<T> {

    /// Gets the id for the Entity being constructed.
    fn id(&self) -> EntityId;

    /// Indicates to the graph that this entity accesses the virtual resource at
    /// the construction phase.
    ///
    /// Since the construction phase can happen in parallel on the CPU, this is
    /// used to indicate to the graph that entities in the graph can not be
    /// constructed in parallel.
    fn use_virtual(
        &mut self,
        id: VirtualId,
        usage: VirtualUsage,
    );

    // /// Create syncpoint on the provided stages of the current entity.
    // /// If the provided `stages` are not valid for this entity type, a warning
    // /// will be emitted, and the SyncPoint will be created at the latest
    // /// possible stage, `END_OF_PIPE`.
    // ///
    // /// If a sync point is created with a `stages` of `0`, this will be a
    // /// virtual sync point. A virtual sync point will NEVER cause any
    // /// synchronization to happen on the GPU, but can be used to mark entites
    // /// as roots in the render graph, see `sync_point_root`.
    // fn sync_point_create(
    //     &mut self,
    //     stages: hal::pso::PipelineStage,
    // ) -> SyncPoint;

    // /// Insert a pipeline dependency on the given sync point.
    // /// Please note that this is NOT sufficient for memory synchronization!
    // /// If you're not sure what this does you are very likely using it wrong.
    // fn sync_point_depend(
    //     &mut self,
    //     dependency: SyncPoint,
    //     stages: hal::pso::PipelineStage,
    // );

    /// Declare usage of the image by the entity.
    ///
    /// If the `usage` involves a write, any subsequent usages will be dependent
    /// on this modification.
    ///
    /// One should be careful of this causing entities to be scheduled in a
    /// different order than intended. `Parameter<()>` dependencies between
    /// nodes can be used to signal this order, or you can choose to rely on the
    /// defined order of the graph builder you are using.
    fn use_image(
        &mut self,
        id: ImageId,
        usage: ImageUsage,
    ) -> Result<ImageToken, EntityConstructionError>;

    /// Declare usage of the buffer by the entity.
    ///
    /// If the `usage` involves a write, any subsequent usages will be dependent
    /// on this modification.
    /// One should be careful of this causing entities to be scheduled in a different
    /// order than intended. `Parameter<()>` dependencies between nodes can be used
    /// to signal this order, or you can choose to rely on the defined order of
    /// the graph builder you are using.
    fn use_buffer(
        &mut self,
        id: BufferId,
        usage: BufferUsage,
    ) -> Result<BufferToken, EntityConstructionError>;

}

pub trait GraphCtx<T: SchedulerTypes>: Sized {
    //fn get_parameter<P: Any>(&self, id: Parameter<P>) -> Result<&P, UnsetParameterError>;
    //fn put_parameter<P: Any>(&mut self, id: Parameter<P>, param: P);

    /// Marks the given entity or resource as a root, indicating to the graph
    /// that it and its dependencies are required to be scheduled for running.
    fn mark_root<R: Into<Root>>(&mut self, root: R);

    /// Creates a new virtual resource
    fn create_virtual(&mut self) -> VirtualId;

    /// Create new image owned by graph
    fn create_image(&mut self, info: ImageInfo) -> ImageId;

    /// Provide node owned image into the graph for a single frame.
    /// `image_info` must be of a known kind.
    ///
    ///
    /// If a sync point is provided, the graph will know the image can only be
    /// used after this sync point.
    /// Specifying None here is ONLY safe if the buffer is availible on the device
    /// at submit time for the graph.
    ///
    /// If a `ProvidedImageUsage` is provided, the graph will know the current
    /// state of the image, and might have to perform less synchronization
    /// before use. Supplying None here is always safe.
    ///
    /// Safety:
    /// This MUST be a semaphore from outside of the graph.
    fn provide_image(
        &mut self,
        image_info: ImageInfo,
        image: impl Into<T::Image>,
        acquire: Option<T::Semaphore>,
        provided_image_usage: Option<ProvidedImageUsage>,
    ) -> ImageId;

    /// This does a virtual move of an image resource.
    /// Any usage of the `from` image after this call will result in a graph
    /// validation error.
    ///
    /// # Example
    /// As an example, a common use case of this operation is for presenting to a
    /// swap chain. The swap chain presentation entity would usually be the last
    /// entity in the graph, and would depend on, say, an image buffer from your
    /// render passes. The ImageId that goes through all of your render passes would
    /// be one created with `create_image` at the beginning of the graph.
    ///
    /// The presentation entity could then, at the end of the graph, obtain the
    /// render target and provide it to the graph with `provide_image`. The entity
    /// would then call `move_image(incoming_color_image, swapchain_image)`, and the
    /// graph will then make sure that `incoming_color_image` is backed by your
    /// provided image instead of a random image managed by the graph.
    ///
    /// # Constraints
    /// `from` must be a non-persistent graph managed image resource.
    /// `to` must never be used outside of creation
    /// `from` and `to` must be the same type.
    fn move_image(&mut self, from: ImageId, to: ImageId);

    /// Create new buffer owned by graph
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> BufferId;

    /// Provide node owned buffer into the graph for single frame.
    ///
    /// If a sync point is provided, the graph will know the buffer can only be
    /// used after this sync point.
    /// Specifying None here is ONLY safe if the buffer is availible on the device
    /// at submit time for the graph.
    ///
    /// If a `ProvidedBufferUsage` is provided, the graph will know the current
    /// state of the buffer, and might have to perform less synchronization
    /// before use. Supplying None here is always safe.
    ///
    /// Safety:
    /// This MUST be a semaphore from outside of the graph.
    fn provide_buffer(
        &mut self,
        buffer_info: BufferInfo,
        image: impl Into<T::Buffer>,
        acquire: Option<T::Semaphore>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    ) -> BufferId;

    /// This does a virtual move of an buffer resource.
    /// Any usage of the `from` buffer after this call will result in a graph
    /// validation error.
    ///
    /// # Constraints
    /// `from` must be a non-persistent graph managed buffer resource.
    /// `to` must never be used outside of creation
    /// `from` and `to` must be the same type.
    fn move_buffer(&mut self, from: BufferId, to: BufferId);

    /// This will create a new persistence token for use with persistant resources.
    /// Note that any resource associated with a token will not be freed until
    /// either the graph is dropped, or the token is disposed of.
    fn create_persistence_token<K: PersistentKind>(&mut self) -> PersistenceToken<K>;

    /// Disposes of the given persistence token, and frees any associated resources
    /// at the end of the current graph run. It is safe to use the resource for the
    /// rest of the current run, but any use after the current run will result in a
    /// graph validation error.
    fn dispose_persistence_token<K: PersistentKind>(&mut self, token: PersistenceToken<K>);

    /// Get a sync point where the graph is finished with the given resource.
    fn sync_point_get<A: HasSyncPoint>(&mut self, a: A) -> SyncPoint;

    /// Does not create or insert any dependencies, but creates a new SyncPoint
    /// that is the combination of all the given dependencies.
    /// That is, all dependent SyncPoints will need to have been triggered before
    /// the returned SyncPoint is triggered.
    fn sync_point_and<A1: HasSyncPoint, A2: HasSyncPoint>(
        &mut self,
        a: A1,
        b: A2,
    ) -> SyncPoint;

    /// Generates a `SemaphoreId` for the given dependency.
    /// After constructing the graph, this `SemaphoreId` can be used to obtain
    /// an actual semaphore from the graph, if the given sync point actually
    /// got scheduled.
    ///
    /// TODO: We might want this to take a semaphore to trigger instead?
    /// That might make things more complicated in the case that the syncpoint
    /// didn't actually get scheduled. When things are like this, we have the
    /// opportunity to not give an actual Semaphore from the SemaphoreId.
    fn sync_point_to_semaphore<A: HasSyncPoint>(
        &mut self,
        dependency: A,
    ) -> SemaphoreId;

    /// Generates a `FenceId` for the given sync point.
    /// After constructing the graph, this `FenceId` can be used to obtain
    /// an actual fence from the graph, if the given sync point actually
    /// got scheduled. You may with to also call `sync_point_root` on the same
    /// sync point in order to guarantee it gets scheduled.
    fn sync_point_to_fence<A: HasSyncPoint>(
        &mut self,
        dependency: A,
    ) -> FenceId;

    // /// Executes the given function once the sync point has been reached.
    // /// If the sync point doesn't get scheduled, nothing is called.
    // fn sync_point_on<A: HasSyncPoint, F>(&mut self, fun: F)
    // where
    //     F: FnOnce();

}

//pub struct NodeCtxImpl<'run, B: hal::Backend> {}
//impl<'run, B: hal::Backend> NodeCtxImpl<'run, B> {
//
//    fn pass<F, O>(&mut self, fun: F) -> Result<O, EntityConstructionError>
//    where
//        F: FnOnce(&mut dyn PassEntityCtx<'run, B>) -> Result<O, EntityConstructionError>;
//
//}

//pub trait Node<B: hal::Backend, T: ?Sized>: std::fmt::Debug + Send + Sync {
//
//    /// Construction phase of node, during which the usage of all graph resources
//    /// are declared.
//    ///
//    /// Will only actually be executed if it's a dependency of the root.
//    fn construct<'run>(
//        &'run mut self,
//        ctx: &mut impl NodeCtx<'run, B>,
//        aux: &'run T,
//    ) -> Result<(), NodeConstructionError>;
//
//    unsafe fn dispose(_self: Box<Self>, _factory: &mut Factory<B>, _aux: &T) {}
//
//}
