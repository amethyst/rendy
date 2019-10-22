//! A cache to store and retrieve samplers

use {
    super::Sampler,
    crate::escape::Handle,
    rendy_core::hal::{image::SamplerInfo, Backend},
    std::{
        collections::hash_map::{Entry, HashMap},
        ops::{Deref, DerefMut},
    },
};

/// Sampler cache holds handlers to created samplers.
#[derive(Debug, derivative::Derivative)]
#[derivative(Default(bound = ""))]
pub struct SamplerCache<B: Backend> {
    samplers: HashMap<SamplerInfo, Handle<Sampler<B>>>,
}

impl<B> SamplerCache<B>
where
    B: Backend,
{
    /// Get sampler with specified paramters.
    /// Create new one using closure provided.
    pub fn get(
        &mut self,
        info: SamplerInfo,
        create: impl FnOnce() -> Result<Handle<Sampler<B>>, rendy_core::hal::device::AllocationError>,
    ) -> Result<Handle<Sampler<B>>, rendy_core::hal::device::AllocationError> {
        Ok(match self.samplers.entry(info) {
            Entry::Occupied(occupied) => occupied.get().clone(),
            Entry::Vacant(vacant) => {
                let sampler = create()?;
                vacant.insert(sampler).clone()
            }
        })
    }

    /// Get sampler with specified paramters.
    /// Create new one using closure provided.
    /// Does not lock for writing if sampler exists.
    pub fn get_with_upgradable_lock<R, W, U>(
        read: R,
        upgrade: U,
        info: SamplerInfo,
        create: impl FnOnce() -> Result<Handle<Sampler<B>>, rendy_core::hal::device::AllocationError>,
    ) -> Result<Handle<Sampler<B>>, rendy_core::hal::device::AllocationError>
    where
        R: Deref<Target = Self>,
        W: DerefMut<Target = Self>,
        U: FnOnce(R) -> W,
    {
        if let Some(sampler) = read.samplers.get(&info) {
            return Ok(sampler.clone());
        }
        let sampler = create()?;
        {
            upgrade(read).samplers.insert(info, sampler.clone());
        }
        Ok(sampler)
    }
}
