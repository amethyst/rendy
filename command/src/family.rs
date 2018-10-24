//! Family module docs.

use ash::{version::DeviceV1_0, vk::{CommandPool, Queue, QueueFlags}};
use capability::Capability;
use pool::Pool;

/// Unique family index.
#[derive(Clone, Copy, Debug)]
pub struct FamilyId(pub u32);

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(Clone, Debug)]
pub struct Family<C = QueueFlags> {
    index: FamilyId,
    queues: Vec<Queue>,
    capability: C,
}

impl<C> Family<C> {
    /// Get queues of the family.
    pub fn queues(&mut self) -> &mut [Queue] {
        &mut self.queues
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    pub fn create_pool<D, R>(&self, device: &impl DeviceV1_0, reset: R) -> Pool<CommandPool, C, R> {
        unimplemented!()
    }
}

impl<C> Family<C>
where
    C: Capability,
{
    /// Convert from some `Family<C>` where `C` is something that implements
    /// `Capability`
    pub fn into_flags(self) -> Family<QueueFlags> {
        Family {
            index: self.index,
            queues: self.queues,
            capability: self.capability.into_flags(),
        }
    }

    /// Convert into a `Family<C>` where `C` something that implements
    /// `Capability`
    pub fn from_flags(family: Family<QueueFlags>) -> Option<Self> {
        if let Some(capability) = C::from_flags(family.capability) {
            Some(Family {
                index: family.index,
                queues: family.queues,
                capability,
            })
        } else {
            None
        }
    }
}

/// Collection of all families.
#[derive(Clone, Debug)]
pub struct Families {
    families: Vec<Family<QueueFlags>>,
}

impl Families {
    /// Create a new Families collection that is empty
    pub fn new() -> Self {
        Families {
            families: Vec::new(),
        }
    }

    /// Add a family to the `Families` group
    pub fn add_family<C>(&mut self, family: Family<C>)
    where
        C: Capability,
    {
        self.families.push(family.into_flags());
    }
}
