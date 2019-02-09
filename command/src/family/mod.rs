//! Family module docs.

mod queue;
mod submission;

use crate::{
    buffer::Reset,
    capability::{Capability, Supports},
    pool::CommandPool,
};

pub use self::{queue::*, submission::*};

/// Family id.
pub type FamilyId = gfx_hal::queue::QueueFamilyId;

/// Queue id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueueId(pub FamilyId, pub usize);

impl QueueId {
    /// Get family of the queue.
    pub fn family(&self) -> FamilyId {
        self.0
    }

    /// Get index of the queue.
    pub fn index(&self) -> usize {
        self.1
    }
}

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Family<B: gfx_hal::Backend, C = gfx_hal::QueueType> {
    id: FamilyId,
    queues: Vec<Queue<B>>,
    // min_image_transfer_granularity: gfx_hal::image::Extent,
    capability: C,
    relevant: relevant::Relevant,
}

impl<B> Family<B>
where
    B: gfx_hal::Backend,
{
    /// Query queue family from device.
    ///
    /// # Safety
    ///
    /// This function shouldn't be used more then once with the same parameters.
    /// Raw queue handle queried from device can make `Family` usage invalid.
    /// `family` must be one of the family indices used during `device` creation.
    /// `properties` must be the properties retuned for queue family from physical device.
    pub unsafe fn from_device(
        queues: &mut gfx_hal::queue::Queues<B>,
        id: FamilyId,
        count: usize,
        family: &impl gfx_hal::queue::QueueFamily,
    ) -> Self {
        Family {
            id,
            queues: {
                let queues = queues.take_raw(id).expect("");
                assert_eq!(queues.len(), count);
                queues
                    .into_iter()
                    .enumerate()
                    .map(|(index, queue)| Queue::new(queue, QueueId(id, index)))
                    .collect()
            },
            // min_image_transfer_granularity: properties.min_image_transfer_granularity,
            capability: family.queue_type(),
            relevant: relevant::Relevant,
        }
    }
}

impl<B, C> Family<B, C>
where
    B: gfx_hal::Backend,
{
    /// Get id of the family.
    pub fn id(&self) -> FamilyId {
        self.id
    }

    /// Get queues of the family.
    pub fn queues(&self) -> &[Queue<B>] {
        &self.queues
    }

    /// Get queues of the family.
    pub fn queues_mut(&mut self) -> &mut [Queue<B>] {
        &mut self.queues
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    pub fn create_pool<R>(
        &self,
        device: &impl gfx_hal::Device<B>,
    ) -> Result<CommandPool<B, C, R>, gfx_hal::device::OutOfMemory>
    where
        R: Reset,
        C: Capability,
    {
        let reset = R::default();
        let pool = unsafe {
            // Is this family belong to specified device.
            let raw = device.create_command_pool(self.id, reset.flags())?;

            CommandPool::from_raw(raw, self.capability, reset, self.id)
        };

        Ok(pool)
    }

    /// Get family capability.
    pub fn capability(&self) -> C
    where
        C: Capability,
    {
        self.capability
    }

    /// Dispose of queue family container.
    pub fn dispose(self) {
        for queue in self.queues {
            queue.wait_idle().unwrap();
        }

        self.relevant.dispose();
    }

    /// Convert capability from type-level to value-level.
    pub fn with_queue_type(self) -> Family<B, gfx_hal::QueueType>
    where
        C: Capability,
    {
        Family {
            id: self.id,
            queues: self.queues,
            // min_image_transfer_granularity: self.min_image_transfer_granularity,
            capability: self.capability.into_queue_type(),
            relevant: self.relevant,
        }
    }

    /// Convert capability into type-level one.
    ///
    pub fn with_capability<U>(self) -> Result<Family<B, U>, Self>
    where
        C: Supports<U>,
    {
        if let Some(capability) = self.capability.supports() {
            Ok(Family {
                id: self.id,
                queues: self.queues,
                // min_image_transfer_granularity: self.min_image_transfer_granularity,
                capability,
                relevant: self.relevant,
            })
        } else {
            Err(self)
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
/// `properties` must contain properties retuned for queue family from physical device for each family id yielded by `families`.
pub unsafe fn families_from_device<B>(
    queues: &mut gfx_hal::queue::Queues<B>,
    families: impl IntoIterator<Item = (FamilyId, usize)>,
    queue_types: &[impl gfx_hal::queue::QueueFamily],
) -> Vec<Family<B>>
where
    B: gfx_hal::Backend,
{
    families
        .into_iter()
        .map(|(index, count)| Family::from_device(queues, index, count, &queue_types[index.0]))
        .collect()
}
