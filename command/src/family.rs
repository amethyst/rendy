//! Family module docs.

use crate::{
    buffer::{Level, NoIndividualReset, Reset},
    capability::Capability,
    pool::{CommandPool, OwningCommandPool},
};

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Family<B: gfx_hal::Backend, C: Capability = gfx_hal::QueueType> {
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
        family: gfx_hal::queue::QueueFamilyId,
        queue_count: u32,
        queue_type: gfx_hal::QueueType,
    ) -> Self {
        Family {
            index: family,
            queues: {
                let queues = queues.take_raw(family).expect("");
                assert_eq!(queues.len(), queue_count as usize);
                queues
            },
            // min_image_transfer_granularity: properties.min_image_transfer_granularity,
            capability: queue_type,
            relevant: relevant::Relevant,
        }
    }
}

impl<B, C: Capability> Family<B, C>
where
    B: gfx_hal::Backend,
{
    /// Get id of the family.
    pub fn index(&self) -> gfx_hal::queue::QueueFamilyId {
        self.index
    }

    /// Get queues of the family.
    pub fn queues(&mut self) -> &mut [B::CommandQueue] {
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
    {
        self.create_pool(device, NoIndividualReset)
            .map(|pool| unsafe { OwningCommandPool::from_inner(pool, level) })
    }

    /// Get family capability.
    pub fn capability(&self) -> C {
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
}

impl<B, C> Family<B, C>
where
    B: gfx_hal::Backend,
    C: Capability,
{
    /// Convert from some `Family<C>` where `C` is something that implements
    /// `Capability`
    pub fn into_flags(self) -> Family<B, gfx_hal::QueueType> {
        Family {
            index: self.index,
            queues: self.queues,
            // min_image_transfer_granularity: self.min_image_transfer_granularity,
            capability: self.capability.into_flags(),
            relevant: self.relevant,
        }
    }

    /// Convert into a `Family<C>` where `C` something that implements
    /// `Capability`
    pub fn from_flags(family: Family<B, gfx_hal::QueueType>) -> Option<Self> {
        if let Some(capability) = C::from_flags(family.capability) {
            Some(Family {
                index: family.index,
                queues: family.queues,
                // min_image_transfer_granularity: family.min_image_transfer_granularity,
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
pub unsafe fn families_from_device<B>(
    queues: &mut gfx_hal::queue::Queues<B>,
    families: impl IntoIterator<Item = (gfx_hal::queue::QueueFamilyId, u32)>,
    queue_types: &[gfx_hal::QueueType],
) -> Vec<Family<B>>
where
    B: gfx_hal::Backend,
{
    families
        .into_iter()
        .map(|(index, queue_count)| {
            Family::from_device(queues, index, queue_count, queue_types[index.0 as usize])
        }).collect()
}
