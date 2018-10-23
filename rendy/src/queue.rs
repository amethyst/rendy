use command::{CapabilityFlags, Families, Family};

/// Trait that represents some method to select a queue family.
pub trait QueuesPicker {
    fn pick_queues<Q>(
        &self,
        families: Vec<Families<Q>>,
    ) -> Result<(Family<Q, CapabilityFlags>, u32), ()>;
}
