use command::{CapabilityFlags, FamilyId};

/// Trait that represents some method to select a queue family.
pub trait QueuesPicker {
    fn pick_queues(&self) -> Result<(FamilyId, u32), ()>;
}
