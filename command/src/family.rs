//! Family module docs.

use capability::CapabilityFlags;
use device::Device;
use pool::Pool;
use queue::Queue;

/// Unique family index.
#[derive(Clone, Copy, Debug)]
pub struct FamilyId(pub u32);

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(Clone, Debug)]
pub struct Family<Q, C> {
    index: FamilyId,
    queues: Vec<Queue<Q, C>>,
    capability: C,
}

impl<Q, C> Family<Q, C> {
    /// Get queues of the family.
    pub fn queues(&mut self) -> &mut [Queue<Q, C>] {
        &mut self.queues
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    pub fn create_pool<D, R>(&mut self, device: &mut D, reset: R) -> Pool<D::CommandPool, C, R>
    where
        D: Device,
    {
        unimplemented!()
    }
}

/// Collection of all families.
#[derive(Clone, Debug)]
pub struct Families<Q> {
    families: Vec<Family<Q, CapabilityFlags>>,
}
