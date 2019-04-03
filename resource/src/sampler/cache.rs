//! A cache to store and retrieve samplers

use {
    super::Sampler,
    crate::escape::Handle,
    gfx_hal::{image::SamplerInfo, Backend},
    std::{
        collections::hash_map::{Entry, HashMap},
        ops::{Deref, DerefMut},
    },
};

#[derive(Debug, derivative::Derivative)]
#[derivative(Default(bound = ""))]
pub struct SamplerCache<B: Backend> {
    samplers: HashMap<SamplerInfo, Handle<Sampler<B>>>,
}

impl<B> SamplerCache<B>
where
    B: Backend,
{
    pub fn get(
        &mut self,
        info: SamplerInfo,
        create: impl FnOnce() -> Result<Handle<Sampler<B>>, gfx_hal::device::AllocationError>,
    ) -> Result<Handle<Sampler<B>>, gfx_hal::device::AllocationError> {
        Ok(match self.samplers.entry(info) {
            Entry::Occupied(occupied) => occupied.get().clone(),
            Entry::Vacant(vacant) => {
                let sampler = create()?;
                vacant.insert(sampler).clone()
            }
        })
    }

    pub fn get_with_upgradable_lock<R, W, U>(
        read: R,
        upgrade: U,
        info: SamplerInfo,
        create: impl FnOnce() -> Result<Handle<Sampler<B>>, gfx_hal::device::AllocationError>,
    ) -> Result<Handle<Sampler<B>>, gfx_hal::device::AllocationError>
    where
        R: Deref<Target = Self>,
        W: DerefMut<Target = Self>,
        U: FnOnce(R) -> W,
    {
        if let Some(sampler) = read.samplers.get(&info) {
            return Ok(sampler.clone());
        }

        upgrade(read).get(info, create)
    }
}
