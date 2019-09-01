//! Sampler creation-info and wrappers.

mod cache;

use {
    crate::util::{device_owned, Device, DeviceId},
    gfx_hal::{image::SamplerInfo, Backend, device::Device as _},
    relevant::Relevant,
};

pub use crate::sampler::cache::SamplerCache;

/// Generic sampler resource wrapper.
#[derive(Debug)]
pub struct Sampler<B: Backend> {
    device: DeviceId,
    raw: B::Sampler,
    info: SamplerInfo,
    relevant: Relevant,
}

device_owned!(Sampler<B>);

impl<B> Sampler<B>
where
    B: Backend,
{
    /// Create new sampler.
    pub fn create(
        device: &Device<B>,
        info: SamplerInfo,
    ) -> Result<Self, gfx_hal::device::AllocationError> {
        // TODO: Check info is valid.
        let raw = unsafe { device.create_sampler(info.clone()) }?;
        Ok(Sampler {
            device: device.id(),
            raw,
            info,
            relevant: Relevant,
        })
    }

    /// Destroy sampler resource.
    pub unsafe fn dispose(self, device: &Device<B>) {
        self.assert_device_owner(device);
        device.destroy_sampler(self.raw);
        self.relevant.dispose();
    }

    /// Get reference to raw sampler resource.
    pub fn raw(&self) -> &B::Sampler {
        &self.raw
    }

    /// Get mutable reference to raw sampler resource.
    pub unsafe fn raw_mut(&mut self) -> &mut B::Sampler {
        &mut self.raw
    }
}
