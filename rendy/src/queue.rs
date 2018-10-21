use rendy_command::{CapabilityFlags, FamilyId};

pub trait QueuesPicker {
    fn pick_queues(&self) -> Result<(FamilyId, u32), ()>;
}
