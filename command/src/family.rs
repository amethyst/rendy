//! Family module docs.

use ash::{version::DeviceV1_0, vk};

use failure::Error;
use relevant::Relevant;

use crate::{
    buffer::{Level, NoIndividualReset, Reset},
    capability::Capability,
    pool::{CommandPool, OwningCommandPool},
};

/// Unique family index.
#[derive(Clone, Copy, Debug)]
pub struct FamilyIndex(pub u32);

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(Clone, Debug)]
pub struct Family<C: Capability = vk::QueueFlags> {
    index: FamilyIndex,
    queues: Vec<vk::Queue>,
    min_image_transfer_granularity: vk::Extent3D,
    capability: C,
    relevant: Relevant,
}

impl Family {
    /// Query queue family from device.
    ///
    /// # Safety
    ///
    /// This function shouldn't be used more then once with the same parameters.
    /// Raw queue handle queried from device can make `Family` usage invalid.
    /// `family` must be one of the family indices used during `device` creation.
    /// `queues` must be equal to number of queues specified for `family` during `device` creation.
    /// `properties` must be the properties retuned for queue family from physical device.
    pub unsafe fn from_device(
        device: &impl DeviceV1_0,
        family: FamilyIndex,
        queues: u32,
        properties: &vk::QueueFamilyProperties,
    ) -> Self {
        Family {
            index: family,
            queues: (0..queues)
                .map(|queue_index| device.get_device_queue(family.0, queue_index))
                .collect(),
            min_image_transfer_granularity: properties.min_image_transfer_granularity,
            capability: properties.queue_flags,
            relevant: Relevant,
        }
    }
}

impl<C: Capability> Family<C> {
    /// Get id of the family.
    pub fn index(&self) -> FamilyIndex {
        self.index
    }

    /// Get queues of the family.
    pub fn queues(&mut self) -> &mut [vk::Queue] {
        &mut self.queues
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    pub fn create_pool<R>(
        &self,
        device: &impl DeviceV1_0,
        reset: R,
    ) -> Result<CommandPool<C, R>, Error>
    where
        R: Reset,
    {
        let pool = unsafe {
            // Is this family belong to specified device.
            let raw = device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(self.index.0)
                    .flags(reset.flags())
                    .build(),
                None,
            )?;

            CommandPool::from_raw(raw, self.capability, reset, self.index)
        };

        Ok(pool)
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    /// Created pool owns its command buffers.
    pub fn create_owning_pool<L>(
        &self,
        device: &impl DeviceV1_0,
        level: L,
    ) -> Result<OwningCommandPool<C, L>, Error>
    where
        L: Level,
    {
        self.create_pool(device, NoIndividualReset)
            .map(|pool| unsafe { OwningCommandPool::from_inner(pool, level) })
    }

    /// Get family capability.
    pub fn capability(&self) -> C {
        self.capability
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

impl<C> Family<C>
where
    C: Capability,
{
    /// Convert from some `Family<C>` where `C` is something that implements
    /// `Capability`
    pub fn into_flags(self) -> Family<vk::QueueFlags> {
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
    pub fn from_flags(family: Family<vk::QueueFlags>) -> Option<Self> {
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

/// Query queue families from device.
///
/// # Safety
///
/// This function shouldn't be used more then once with same parameters.
/// Raw queue handle queried from device can make returned `Family` usage invalid.
/// `families` iterator must yeild unique family indices with queue count used during `device` creation.
/// `properties` must contain properties retuned for queue family from physical device for each family index yielded by `families`.
pub unsafe fn families_from_device(
    device: &impl DeviceV1_0,
    families: impl IntoIterator<Item = (FamilyIndex, u32)>,
    properties: &[vk::QueueFamilyProperties],
) -> Vec<Family> {
    families
        .into_iter()
        .map(|(index, queues)| {
            Family::from_device(device, index, queues, &properties[index.0 as usize])
        }).collect()
}
