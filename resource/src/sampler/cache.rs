//! A cache to store and retrieve samplers
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use super::{Sampler, Info};

#[doc(hidden)]
#[derive(Debug)]
pub struct SamplerCache<B: gfx_hal::Backend> {
    // TODO: figure out way to store this better. Perhaps we can clone/copy sampler so we don't need a reference?
    samplers: HashMap<gfx_hal::image::Filter, HashMap<gfx_hal::image::WrapMode, Sampler<B>>>,
}

impl<B> SamplerCache<B>
where
    B: gfx_hal::Backend,
{
    #[doc(hidden)]
    pub fn get(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        filter: gfx_hal::image::Filter,
        wrap_mode: gfx_hal::image::WrapMode
    ) -> Option<&Sampler<B>> {
        match self.samplers.entry(filter) {
            Entry::Occupied(e) => {
                let hashmap = &mut *e.into_mut();
                match hashmap.entry(wrap_mode) {
                    Entry::Occupied(e) => Some(&mut *e.into_mut()),
                    Entry::Vacant(e) => {
                        Some(&*e.insert(SamplerCache::create(device, filter, wrap_mode)))
                    }
                }
            },
            Entry::Vacant(_e) => None,
        }
    }

    fn create(
        device: &impl gfx_hal::Device<B>,
        filter: gfx_hal::image::Filter,
        wrap_mode: gfx_hal::image::WrapMode
    ) -> Sampler<B> {
        let sampler = unsafe {
            device.create_sampler(gfx_hal::image::SamplerInfo::new(filter, wrap_mode)).unwrap()
        };
        Sampler::new(
            Info {
                filter,
                wrap_mode,
            },
            sampler
        )
    }

    #[doc(hidden)]
    pub fn destroy(
        &mut self,
        device: &impl gfx_hal::Device<B>,
    ) {
        for kvp in self.samplers.drain() {
            let mut hash_map = kvp.1;
            for kvp2 in hash_map.drain() {
                let sampler = kvp2.1;
                unsafe { device.destroy_sampler(sampler.raw) };
            }
        }
    }
}

impl<B> Default for SamplerCache<B>
where
    B: gfx_hal::Backend,
{
    fn default() -> Self {
        let mut samplers = HashMap::new();
        samplers.insert(gfx_hal::image::Filter::Linear, HashMap::new());
        samplers.insert(gfx_hal::image::Filter::Nearest, HashMap::new());
        SamplerCache {
            samplers,
        }
    }
}