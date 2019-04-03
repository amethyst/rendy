//! Sampler creation-info and wrappers.

mod cache;

use {
    gfx_hal::{image::SamplerInfo, Backend, Device as _},
    relevant::Relevant,
};

pub use crate::sampler::cache::SamplerCache;

#[derive(Debug)]
pub struct Sampler<B: Backend> {
    raw: B::Sampler,
    info: SamplerInfo,
    relevant: Relevant,
}

impl<B> Sampler<B>
where
    B: Backend,
{
    /// Create new sampler.
    pub unsafe fn create(
        device: &B::Device,
        info: SamplerInfo,
    ) -> Result<Self, gfx_hal::device::AllocationError> {
        let raw = device.create_sampler(info.clone())?;
        Ok(Sampler {
            raw,
            info,
            relevant: Relevant,
        })
    }

    pub unsafe fn dispose(self, device: &B::Device) {
        device.destroy_sampler(self.raw);
        self.relevant.dispose();
    }

    pub fn raw(&self) -> &B::Sampler {
        &self.raw
    }

    pub unsafe fn raw_mut(&mut self) -> &mut B::Sampler {
        &mut self.raw
    }
}
