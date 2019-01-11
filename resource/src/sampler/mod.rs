//! Sampler creation-info and wrappers.

mod cache;

pub use crate::sampler::cache::SamplerCache;

// Image view info
#[derive(Clone, Copy, Debug)]
#[doc(hidden)]
pub struct Info {
    filter: gfx_hal::image::Filter,
    wrap_mode: gfx_hal::image::WrapMode,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Sampler<B: gfx_hal::Backend> {
    raw: B::Sampler,
    info: Info,
}

impl<B> Sampler<B>
where
    B: gfx_hal::Backend,
{
    #[doc(hidden)]
    pub fn new(info: Info, raw: B::Sampler) -> Self {
        Sampler {
            raw,
            info,
        }
    }

    #[doc(hidden)]
    pub fn raw(&self) -> &B::Sampler {
        &self.raw
    }
}