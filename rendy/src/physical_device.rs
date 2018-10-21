use device::Device;
use rendy_command::{Families, FamilyId};

pub trait PhysicalDevice<D: Device>: Sized {
    fn open(&self, family: FamilyId, count: u32) -> Result<(D, Families<D::CommandQueue>), ()>;
}
