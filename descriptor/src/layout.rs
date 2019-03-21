use crate::ranges::DescriptorRanges;

pub use gfx_hal::{
    device::OutOfMemory,
    pso::{DescriptorRangeDesc, DescriptorSetLayoutBinding, DescriptorType},
    Backend, Device,
};

#[derive(Debug)]
pub struct DescriptorSetLayout<B: Backend> {
    raw: B::DescriptorSetLayout,
    bindings: Vec<DescriptorSetLayoutBinding>,
    relevant: relevant::Relevant,
}

impl<B> DescriptorSetLayout<B>
where
    B: Backend,
{
    pub fn create(
        device: &impl Device<B>,
        bindings: Vec<DescriptorSetLayoutBinding>,
    ) -> Result<Self, OutOfMemory> {
        log::trace!("Creating new layout with bindings: {:?}", bindings);
        let raw = unsafe {
            device.create_descriptor_set_layout(&bindings, std::iter::empty::<B::Sampler>())
        }?;
        Ok(DescriptorSetLayout {
            raw,
            bindings,
            relevant: relevant::Relevant,
        })
    }

    pub unsafe fn dispose(self, device: &impl Device<B>) {
        self.relevant.dispose();
        device.destroy_descriptor_set_layout(self.raw);
    }

    pub fn ranges(&self) -> DescriptorRanges {
        DescriptorRanges::from_bindings(&self.bindings)
    }

    pub fn raw(&self) -> &B::DescriptorSetLayout {
        &self.raw
    }

    pub fn bindings(&self) -> &[DescriptorSetLayoutBinding] {
        &self.bindings
    }
}
