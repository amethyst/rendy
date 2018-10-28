//! Family module docs.

use ash::{version::DeviceV1_0, vk::{CommandPool, Queue, QueueFlags, Extent3D, QueueFamilyProperties}};

use relevant::Relevant;

use crate::{
    capability::Capability,
    pool::Pool,
};

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
    min_image_transfer_granularity: Extent3D,
    capability: C,
    relevant: Relevant,
}

impl Family {
    /// Get queue family from device.
    pub unsafe fn from_device(device: &impl DeviceV1_0, index: FamilyId, queues: u32, properties: &QueueFamilyProperties) -> Self {
        Family {
            index,
            queues: (0..queues).map(|queue_index| device.get_device_queue(index.0, queue_index)).collect(),
            min_image_transfer_granularity: properties.min_image_transfer_granularity,
            capability: properties.queue_flags,
            relevant: Relevant,
        }
    }

    /// Dispose of queue family container.
    pub fn dispose(self, device: &impl DeviceV1_0) {
        for queue in self.queues {
            unsafe {
                let _ = device.queue_wait_idle(queue);
            }
        }

        self.relevant.dispose();
    }
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
            min_image_transfer_granularity: self.min_image_transfer_granularity,
            capability: self.capability.into_flags(),
            relevant: self.relevant,
        }
    }

    /// Convert into a `Family<C>` where `C` something that implements
    /// `Capability`
    pub fn from_flags(family: Family<QueueFlags>) -> Option<Self> {
        if let Some(capability) = C::from_flags(family.capability) {
            Some(Family {
                index: family.index,
                queues: family.queues,
                min_image_transfer_granularity: family.min_image_transfer_granularity,
                capability,
                relevant: family.relevant,
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

    /// Get queue families from device.
    pub unsafe fn from_device(device: &impl DeviceV1_0, families: impl IntoIterator<Item = (FamilyId, u32)>, properties: &[QueueFamilyProperties]) -> Self {
        Families {
            families: families.into_iter().map(|(index, queues)| Family::from_device(device, index, queues, &properties[index.0 as usize])).collect()
        }
    }

    /// Dispose of queue family containers.
    pub fn dispose(self, device: &impl DeviceV1_0) {
        for family in self.families {
            family.dispose(device);
        }

        unsafe {
            device.device_wait_idle();
        }
    }
}
