//! A cache to store and retrieve samplers
use super::{Info, Sampler};
use crate::escape::Terminal;
use std::collections::HashMap;

#[doc(hidden)]
#[derive(Debug)]
pub struct SamplerCache<B: gfx_hal::Backend> {
    samplers: HashMap<(gfx_hal::image::Filter, gfx_hal::image::WrapMode), Sampler<B>>,
    raw_samplers: Terminal<B::Sampler>,
}

impl<B> SamplerCache<B>
where
    B: gfx_hal::Backend,
{
    pub fn get(
        &mut self,
        device: &impl gfx_hal::Device<B>,
        filter: gfx_hal::image::Filter,
        wrap_mode: gfx_hal::image::WrapMode,
    ) -> Sampler<B> {
        let raw_samplers = &self.raw_samplers;
        self.samplers
            .entry((filter, wrap_mode))
            .or_insert_with(|| Self::create(raw_samplers, device, filter, wrap_mode))
            .clone()
    }

    fn create(
        raw_samplers: &Terminal<B::Sampler>,
        device: &impl gfx_hal::Device<B>,
        filter: gfx_hal::image::Filter,
        wrap_mode: gfx_hal::image::WrapMode,
    ) -> Sampler<B> {
        let sampler = unsafe {
            device
                .create_sampler(gfx_hal::image::SamplerInfo::new(filter, wrap_mode))
                .unwrap()
        };
        Sampler::new(Info { filter, wrap_mode }, sampler, raw_samplers)
    }

    pub fn destroy(&mut self, device: &impl gfx_hal::Device<B>) {
        for (_, sampler) in self.samplers.drain() {
            unsafe { device.destroy_sampler(sampler.unescape().unwrap()) };
        }
    }
}

impl<B> Default for SamplerCache<B>
where
    B: gfx_hal::Backend,
{
    fn default() -> Self {
        SamplerCache {
            samplers: HashMap::new(),
            raw_samplers: Terminal::default(),
        }
    }
}
