use rendy_core::hal;
use {
    crate::{
        core::{device_owned, Device, DeviceId},
        descriptor,
        escape::Handle,
    },
    hal::{device::Device as _, pso::DescriptorSetLayoutBinding, Backend},
    relevant::Relevant,
    smallvec::SmallVec,
};

/// Descriptor set layout info.
#[derive(Clone, Debug)]
pub struct DescriptorSetInfo {
    /// Bindings.
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetInfo {
    /// Get descriptor ranges of the layout.
    pub fn ranges(&self) -> descriptor::DescriptorRanges {
        descriptor::DescriptorRanges::from_bindings(&self.bindings)
    }
}

/// Generic descriptor set layout resource wrapper.
#[derive(Debug)]
pub struct DescriptorSetLayout<B: Backend> {
    device: DeviceId,
    raw: B::DescriptorSetLayout,
    info: DescriptorSetInfo,
    relevant: Relevant,
}

device_owned!(DescriptorSetLayout<B>);

impl<B> DescriptorSetLayout<B>
where
    B: Backend,
{
    /// Create new descriptor set layout
    pub unsafe fn create(
        device: &Device<B>,
        info: DescriptorSetInfo,
    ) -> Result<Self, hal::device::OutOfMemory> {
        let raw = device
            .create_descriptor_set_layout(&info.bindings, std::iter::empty::<B::Sampler>())?;

        Ok(DescriptorSetLayout {
            device: device.id(),
            raw,
            info,
            relevant: Relevant,
        })
    }

    /// Destroy descriptor set layout resource.
    pub unsafe fn dispose(self, device: &Device<B>) {
        device.destroy_descriptor_set_layout(self.raw);
        self.relevant.dispose();
    }

    /// Get reference to raw descriptor set layout resource.
    pub fn raw(&self) -> &B::DescriptorSetLayout {
        &self.raw
    }

    /// Get mutable reference to raw descriptor set layout resource.
    pub unsafe fn raw_mut(&mut self) -> &mut B::DescriptorSetLayout {
        &mut self.raw
    }

    /// Get descriptor set layout info.
    pub fn info(&self) -> &DescriptorSetInfo {
        &self.info
    }
}

use derive_more::{Deref, DerefMut};

/// Generic descriptor set resource wrapper.
#[derive(Debug, Deref, DerefMut)]
pub struct DescriptorSet<B: Backend> {
    device: DeviceId,
    #[deref(forward)]
    #[deref_mut(forward)]
    set: descriptor::DescriptorSet<B>,
    layout: Handle<DescriptorSetLayout<B>>,
    relevant: Relevant,
}

impl<B> DescriptorSet<B>
where
    B: Backend,
{
    /// Create new descriptor set.
    pub unsafe fn create(
        device: &Device<B>,
        allocator: &mut descriptor::DescriptorAllocator<B>,
        layout: Handle<DescriptorSetLayout<B>>,
    ) -> Result<Self, hal::device::OutOfMemory> {
        let mut sets = SmallVec::<[_; 1]>::new();

        allocator.allocate(device, layout.raw(), layout.info().ranges(), 1, &mut sets)?;

        Ok(DescriptorSet {
            device: device.id(),
            set: sets.swap_remove(0),
            layout,
            relevant: Relevant,
        })
    }

    /// Create new descriptor sets.
    pub unsafe fn create_many(
        device: &Device<B>,
        allocator: &mut descriptor::DescriptorAllocator<B>,
        layout: Handle<DescriptorSetLayout<B>>,
        count: u32,
        extend: &mut impl Extend<Self>,
    ) -> Result<(), hal::device::OutOfMemory> {
        let mut sets = SmallVec::<[_; 32]>::new();

        allocator.allocate(
            device,
            layout.raw(),
            layout.info().ranges(),
            count,
            &mut sets,
        )?;

        extend.extend(sets.into_iter().map(|set| DescriptorSet {
            device: device.id(),
            set,
            layout: layout.clone(),
            relevant: Relevant,
        }));

        Ok(())
    }

    /// Destroy descriptor set resource.
    pub unsafe fn dispose(self, allocator: &mut descriptor::DescriptorAllocator<B>) {
        allocator.free(Some(self.set));
        self.relevant.dispose();
    }

    /// Get layout of descriptor set.
    pub fn layout(&mut self) -> &Handle<DescriptorSetLayout<B>> {
        &self.layout
    }
}
