use {
    crate::{
        descriptor,
        escape::{Escape, KeepAlive, Terminal},
    },
    gfx_hal::Backend,
    std::ops::{Deref, DerefMut},
};

/// Descriptor set object wrapper.
#[derive(Debug)]
pub struct DescriptorSet<B: Backend> {
    escape: Escape<(descriptor::DescriptorSet<B>, KeepAlive)>,
}

impl<B> DescriptorSet<B>
where
    B: Backend,
{
    /// Wrap a descriptor set.
    ///
    /// # Safety
    ///
    /// `terminal` will receive descriptor set upon drop, it must free descriptor set properly.
    ///
    pub unsafe fn new(
        set: descriptor::DescriptorSet<B>,
        layout: &DescriptorSetLayout<B>,
        terminal: &Terminal<(descriptor::DescriptorSet<B>, KeepAlive)>,
    ) -> Self {
        DescriptorSet {
            escape: terminal.escape((set, layout.keep_alive())),
        }
    }

    /// This will return `None` and would be equivalent to dropping
    /// if there are `KeepAlive` created from this `DescriptorSet.
    pub fn unescape(self) -> Option<(descriptor::DescriptorSet<B>, KeepAlive)> {
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
    /// Raw descriptor set handler should not be used to violate this object valid usage.
    pub fn raw(&self) -> &B::DescriptorSet {
        self.escape.0.raw()
    }
}

#[derive(Debug)]
pub struct DescriptorSetLayout<B: Backend> {
    escape: Escape<descriptor::DescriptorSetLayout<B>>,
}

impl<B> DescriptorSetLayout<B>
where
    B: Backend,
{
    /// Wrap a descriptor set layout.
    ///
    /// # Safety
    ///
    /// `terminal` will receive descriptor set layout upon drop, it must free descriptor set layout properly.
    ///
    pub unsafe fn new(
        layout: descriptor::DescriptorSetLayout<B>,
        terminal: &Terminal<descriptor::DescriptorSetLayout<B>>,
    ) -> Self {
        DescriptorSetLayout {
            escape: terminal.escape(layout),
        }
    }

    /// This will return `None` and would be equivalent to dropping
    /// if there are `KeepAlive` created from this `DescriptorSetLayout.
    pub fn unescape(self) -> Option<descriptor::DescriptorSetLayout<B>> {
        Escape::unescape(self.escape)
    }

    /// Creates [`KeepAlive`] handler to extend descriptor set layout lifetime.
    ///
    /// [`KeepAlive`]: struct.KeepAlive.html
    pub fn keep_alive(&self) -> KeepAlive {
        Escape::keep_alive(&self.escape)
    }

    /// Get raw descriptor set layout handle.
    ///
    /// # Safety
    ///
    /// Raw descriptor set layout handler should not be used to violate this object valid usage.
    pub fn raw(&self) -> &B::DescriptorSetLayout {
        self.escape.raw()
    }
}

impl<B: Backend> Deref for DescriptorSetLayout<B> {
    type Target = descriptor::DescriptorSetLayout<B>;
    fn deref(&self) -> &descriptor::DescriptorSetLayout<B> {
        &*self.escape
    }
}

impl<B: Backend> DerefMut for DescriptorSetLayout<B> {
    fn deref_mut(&mut self) -> &mut descriptor::DescriptorSetLayout<B> {
        &mut *self.escape
    }
}
