//! Sampler creation-info and wrappers.

mod cache;

use crate::escape::{EscapeShared, KeepAlive, Terminal};

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
    escape: EscapeShared<B::Sampler>,
    info: Info,
}

impl<B> Clone for Sampler<B>
where
    B: gfx_hal::Backend,
{
    fn clone(&self) -> Self {
        Sampler {
            escape: self.escape.clone(),
            info: self.info.clone(),
        }
    }
}

impl<B> Sampler<B>
where
    B: gfx_hal::Backend,
{
    pub(crate) fn new(info: Info, raw: B::Sampler, terminal: &Terminal<B::Sampler>) -> Self {
        Sampler {
            escape: terminal.escape_shared(raw),
            info,
        }
    }

    pub(crate) fn unescape(self) -> Option<B::Sampler> {
        EscapeShared::unescape(self.escape)
    }

    /// Creates [`KeepAlive`] handler to extend image lifetime.
    ///
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        EscapeShared::keep_alive(&self.escape)
    }

    pub fn raw(&self) -> &B::Sampler {
        &self.escape
    }
}
