use crate::{
    descriptor,
    escape::{Escape, KeepAlive, Terminal},
};

/// Descriptor set object wrapper.
#[derive(Debug)]
pub struct DescriptorSet<B: gfx_hal::Backend> {
    escape: Escape<descriptor::DescriptorSet<B>>,
}

impl<B> DescriptorSet<B>
where
    B: gfx_hal::Backend,
{
    /// Wrap a descriptor set.
    ///
    /// # Safety
    ///
    /// `terminal` will receive descriptor set upon drop, it must free descriptor set properly.
    ///
    pub unsafe fn new(
        set: descriptor::DescriptorSet<B>,
        terminal: &Terminal<descriptor::DescriptorSet<B>>,
    ) -> Self {
        DescriptorSet {
            escape: terminal.escape(set),
        }
    }

    /// This will return `None` and would be equivalent to dropping
    /// if there are `KeepAlive` created from this `DescriptorSet.
    pub fn unescape(self) -> Option<descriptor::DescriptorSet<B>> {
        Escape::unescape(self.escape)
    }

    /// Creates [`KeepAlive`] handler to extend descriptor set lifetime.
    ///
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        Escape::keep_alive(&self.escape)
    }

    /// Get raw descriptor set handle.
    ///
    /// # Safety
    ///
    /// Raw descriptor set handler should not be usage to violate this object valid usage.
    pub fn raw(&self) -> &B::DescriptorSet {
        self.escape.raw()
    }
}
