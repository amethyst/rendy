//! Family module docs.

use crate::{
    buffer::{Level, NoIndividualReset, Reset},
    capability::{Capability, Supports},
    pool::{CommandPool, OwningCommandPool},
};

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Family<B: gfx_hal::Backend, C = gfx_hal::QueueType> {
    index: gfx_hal::queue::QueueFamilyId,
    #[derivative(Debug = "ignore")] queues: Vec<B::CommandQueue>,
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
        index: gfx_hal::queue::QueueFamilyId,
        count: usize,
        family: &impl gfx_hal::queue::QueueFamily,
    ) -> Self {
        Family {
            index,
            queues: {
                let queues = queues.take_raw(index).expect("");
                assert_eq!(queues.len(), count);
                queues
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
    pub fn index(&self) -> gfx_hal::queue::QueueFamilyId {
        self.index
    }

    /// Get queues of the family.
    pub fn queues(&self) -> &[impl gfx_hal::queue::RawCommandQueue<B>] {
        &self.queues
    }

    /// Get queues of the family.
    pub fn queues_mut(&mut self) -> &mut [impl gfx_hal::queue::RawCommandQueue<B>] {
        &mut self.queues
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    pub fn create_pool<R>(
        &self,
        device: &impl gfx_hal::Device<B>,
        reset: R,
    ) -> Result<CommandPool<B, C, R>, gfx_hal::device::OutOfMemory>
    where
        R: Reset,
        C: Capability,
    {
        let pool = unsafe {
            // Is this family belong to specified device.
            let raw = device.create_command_pool(
                self.index,
                reset.flags(),
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
        device: &impl gfx_hal::Device<B>,
        level: L,
    ) -> Result<OwningCommandPool<B, C, L>, gfx_hal::device::OutOfMemory>
    where
        L: Level,
        C: Capability,
    {
        self.create_pool(device, NoIndividualReset)
            .map(|pool| unsafe { OwningCommandPool::from_inner(pool, level) })
    }

    /// Get family capability.
    pub fn capability(&self) -> C
    where
        C: Capability
    {
        self.capability
    }

    /// Dispose of queue family container.
    pub fn dispose(self, device: &impl gfx_hal::Device<B>) {
        for queue in self.queues {
            gfx_hal::queue::RawCommandQueue::wait_idle(&queue)
                .unwrap();
        }

        self.relevant.dispose();
    }

    /// Convert from some `Family<C>` where `C` is something that implements
    /// `Capability`
    pub fn with_capability_value(self) -> Family<B, gfx_hal::QueueType>
    where
        C: Capability,
    {
        Family {
            index: self.index,
            queues: self.queues,
            // min_image_transfer_granularity: self.min_image_transfer_granularity,
            capability: self.capability.into_queue_type(),
            relevant: self.relevant,
        }
    }

    /// Convert into a `Family<C>` where `C` something that implements
    /// `Capability`
    pub fn with_capability<U>(self) -> Result<Family<B, U>, Self>
    where
        C: Supports<U>,
    {
        if let Some(capability) = self.capability.supports() {
            Ok(Family {
                index: self.index,
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
/// `properties` must contain properties retuned for queue family from physical device for each family index yielded by `families`.
pub unsafe fn families_from_device<B>(
    queues: &mut gfx_hal::queue::Queues<B>,
    families: impl IntoIterator<Item = (gfx_hal::queue::QueueFamilyId, usize)>,
    queue_types: &[impl gfx_hal::queue::QueueFamily],
) -> Vec<Family<B>>
where
    B: gfx_hal::Backend,
{
    families
        .into_iter()
        .map(|(index, count)| {
            Family::from_device(queues, index, count, &queue_types[index.0])
        }).collect()
}
