use cranelift_entity::entity_impl;

use crate::{
    ImageId, BufferId,
};

/// This is a virtual synchronization point in the graph execution.
/// It is not backed by any particular synchronization primitive,
/// instead the graph scheduler ensures that all required
/// synchronization is performed with the best possible method.
///
/// A `SyncPoint` can be triggered once in execution of a graph,
/// at which point everything that waits on it can commence
/// execution.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyncPoint(pub(crate) u32);
entity_impl!(SyncPoint, "sync_point");

#[derive(Debug, Copy, Clone)]
pub enum SyncPointRef {
    SyncPoint(SyncPoint),
    Image(ImageId),
    Buffer(BufferId),
}

/// The entity is something which has an implicit `SyncPoint` in the
/// graph.
/// The `SyncPoint` for an entity can, but should not be expected to
/// be constant over the construction of a graph.
/// An example here is Images and Buffers which get a new SyncPoint
/// every time they are accessed.
pub trait HasSyncPoint: Sized {
    fn into_sync_point(&self) -> SyncPointRef;
}

impl HasSyncPoint for SyncPointRef {
    fn into_sync_point(&self) -> SyncPointRef {
        *self
    }
}
impl HasSyncPoint for SyncPoint {
    fn into_sync_point(&self) -> SyncPointRef {
        SyncPointRef::SyncPoint(*self)
    }
}
impl HasSyncPoint for ImageId {
    fn into_sync_point(&self) -> SyncPointRef {
        SyncPointRef::Image(*self)
    }
}
impl HasSyncPoint for BufferId {
    fn into_sync_point(&self) -> SyncPointRef {
        SyncPointRef::Buffer(*self)
    }
}
