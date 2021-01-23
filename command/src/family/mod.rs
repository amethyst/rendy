//! Family module docs.

mod queue;
mod submission;

use {
    crate::{
        buffer::Reset,
        capability::{Capability, QueueType, Supports},
        core::{device_owned, Device, DeviceId},
        pool::CommandPool,
    },
    rendy_core::hal::Backend,
};

pub use self::{queue::*, submission::*};

/// Family id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FamilyId {
    /// Family id within device.
    pub index: usize,

    /// Device id.
    pub device: DeviceId,
}

impl From<FamilyId> for rendy_core::hal::queue::QueueFamilyId {
    fn from(id: FamilyId) -> Self {
        rendy_core::hal::queue::QueueFamilyId(id.index)
    }
}

/// Queue id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueueId {
    /// Queue index.
    pub index: usize,

    /// Family id.
    pub family: FamilyId,
}

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(Debug)]
pub struct Family<B: Backend, C = QueueType> {
    id: FamilyId,
    queues: Vec<Queue<B>>,
    // min_image_transfer_granularity: rendy_core::hal::image::Extent,
    capability: C,
}

device_owned!(Family<B, C> @ |f: &Self| f.id.device);

impl<B> Family<B, QueueType>
where
    B: Backend,
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
        queue_groups: &mut Vec<rendy_core::hal::queue::QueueGroup<B>>,
        id: FamilyId,
        _count: usize,
        family: &impl rendy_core::hal::queue::QueueFamily,
    ) -> Self {
        Family {
            id,
            queues: {
                let pos = queue_groups.iter().position(|qg| qg.family.0 == id.index);
                let group = queue_groups.swap_remove(pos.unwrap());
                group
                    .queues
                    .into_iter()
                    .enumerate()
                    .map(|(index, queue)| Queue::new(queue, QueueId { family: id, index }))
                    .collect()
            },
            // min_image_transfer_granularity: properties.min_image_transfer_granularity,
            capability: family.queue_type(),
        }
    }
}

impl<B, C> Family<B, C>
where
    B: Backend,
{
    /// Get id of the family.
    pub fn id(&self) -> FamilyId {
        self.id
    }

    /// Get queue by index
    pub fn queue(&self, index: usize) -> &Queue<B> {
        &self.queues[index]
    }

    /// Get queue by index
    pub fn queue_mut(&mut self, index: usize) -> &mut Queue<B> {
        &mut self.queues[index]
    }

    /// Get queues of the family.
    pub fn as_slice(&self) -> &[Queue<B>] {
        &self.queues
    }

    /// Get queues of the family.
    pub fn as_slice_mut(&mut self) -> &mut [Queue<B>] {
        &mut self.queues
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    pub fn create_pool<R>(
        &self,
        device: &Device<B>,
    ) -> Result<CommandPool<B, C, R>, rendy_core::hal::device::OutOfMemory>
    where
        R: Reset,
        C: Capability,
    {
        unsafe { CommandPool::create(self.id, self.capability, device) }
    }

    /// Get family capability.
    pub fn capability(&self) -> C
    where
        C: Capability,
    {
        self.capability
    }

    /// Convert capability from type-level to value-level.
    pub fn with_queue_type(self) -> Family<B, QueueType>
    where
        C: Capability,
    {
        Family {
            id: self.id,
            queues: self.queues,
            // min_image_transfer_granularity: self.min_image_transfer_granularity,
            capability: self.capability.into_queue_type(),
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
            })
        } else {
            Err(self)
        }
    }
}

/// Collection of queue families of one device.
#[derive(Debug)]
pub struct Families<B: Backend> {
    device: DeviceId,
    families: Vec<Family<B>>,
    families_indices: Vec<usize>,
}

impl<B> Families<B>
where
    B: Backend,
{
    /// Get queue family by id.
    pub fn family(&self, id: FamilyId) -> &Family<B> {
        self.family_by_index(id.index)
    }

    /// Get queue family by index.
    pub fn family_by_index(&self, index: usize) -> &Family<B> {
        &self.families[self.families_indices[index]]
    }

    /// Get queue family by id.
    pub fn family_mut(&mut self, id: FamilyId) -> &mut Family<B> {
        self.family_by_index_mut(id.index)
    }

    /// Get queue family by index.
    pub fn family_by_index_mut(&mut self, index: usize) -> &mut Family<B> {
        &mut self.families[self.families_indices[index]]
    }

    /// Get queue families as slice.
    pub fn as_slice(&self) -> &[Family<B>] {
        &self.families
    }

    /// Get queue families as slice.
    pub fn as_slice_mut(&mut self) -> &mut [Family<B>] {
        &mut self.families
    }

    /// Get id -> index mapping.
    pub fn indices(&self) -> &[usize] {
        &self.families_indices
    }

    /// Find family id matching predicate
    pub fn find<F>(&self, predicate: F) -> Option<FamilyId>
    where
        F: FnMut(&&Family<B>) -> bool,
    {
        self.families.iter().find(predicate).map(Family::id)
    }

    /// Get first matching family id with specified capability
    pub fn with_capability<C: Capability>(&self) -> Option<FamilyId> {
        self.find(|family| Supports::<C>::supports(&family.capability()).is_some())
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
    device: DeviceId,
    queue_groups: &mut Vec<rendy_core::hal::queue::QueueGroup<B>>,
    families: impl IntoIterator<Item = (FamilyId, usize)>,
    queue_types: &[impl rendy_core::hal::queue::QueueFamily],
) -> Families<B>
where
    B: Backend,
{
    let families: Vec<_> = families
        .into_iter()
        .map(|(id, count)| Family::from_device(queue_groups, id, count, &queue_types[id.index]))
        .collect();

    let mut families_indices = vec![!0; families.len()];
    for (index, family) in families.iter().enumerate() {
        families_indices[family.id.index] = index;
    }

    Families {
        device,
        families,
        families_indices,
    }
}
