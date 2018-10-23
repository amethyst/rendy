//! Family module docs.

use capability::{Capability, CapabilityFlags};
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

impl<Q, C> Family<Q, C>
where
    C: Capability,
{
    /// Convert from some `Family<Q, C>` where `C` is something that implements
    /// `Capability`
    pub fn from(family: Self) -> Family<Q, CapabilityFlags> {
        Family {
            index: family.index,
            queues: family
                .queues
                .into_iter()
                .map(|queue| Queue {
                    inner: queue.inner,
                    capability: queue.capability.into_flags(),
                }).collect::<Vec<_>>(),
            capability: family.capability.into_flags(),
        }
    }

    /// Convert into a `Family<Q, C>` where `C` something that implements
    /// `Capability`
    pub fn into(family: Family<Q, CapabilityFlags>) -> Option<Self> {
        if let Some(capability) = C::from_flags(family.capability) {
            Some(Family {
                index: family.index,
                queues: family
                    .queues
                    .into_iter()
                    .map(|queue| Queue {
                        inner: queue.inner,
                        capability: C::from_flags(queue.capability)
                            .expect("Unable to convert queue capability to a CapabilityFlag"),
                    }).collect::<Vec<_>>(),
                capability,
            })
        } else {
            None
        }
    }
}

/// Collection of all families.
#[derive(Clone, Debug)]
pub struct Families<Q> {
    families: Vec<Family<Q, CapabilityFlags>>,
}

impl<Q> Families<Q> {
    /// Create a new Families collection that is empty
    pub fn new() -> Self {
        Families {
            families: Vec::new(),
        }
    }

    /// Add a family to the `Families<Q>` group
    pub fn add_family<C>(&mut self, family: Family<Q, C>)
    where
        C: Capability,
    {
        self.families.push(Family::from(family));
    }
}
