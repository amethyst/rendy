use {
    crate::{
        core::{device_owned, Device, DeviceId},
        descriptor,
        escape::Handle,
    },
    relevant::Relevant,
    rendy_core::hal::{device::Device as _, pso::DescriptorSetLayoutBinding, Backend},
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
    ) -> Result<Self, rendy_core::hal::device::OutOfMemory> {
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
        self.assert_device_owner(device);
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

/// Generic descriptor set resource wrapper.
#[derive(Debug)]
pub struct DescriptorSet<B: Backend> {
    device: DeviceId,
    set: descriptor::DescriptorSet<B>,
    layout: Handle<DescriptorSetLayout<B>>,
    relevant: Relevant,
}

device_owned!(DescriptorSet<B>);

impl<B> DescriptorSet<B>
where
    B: Backend,
{
    /// Create new descriptor set.
    pub unsafe fn create(
        device: &Device<B>,
        allocator: &mut descriptor::DescriptorAllocator<B>,
        layout: Handle<DescriptorSetLayout<B>>,
    ) -> Result<Self, rendy_core::hal::device::OutOfMemory> {
        let mut sets = SmallVec::<[_; 1]>::new();

        allocator.allocate(device, layout.raw(), layout.info().ranges(), 1, &mut sets)?;

        assert_eq!(sets.len() as u32, 1);
        Ok(DescriptorSet {
            device: device.id(),
            set: sets.swap_remove(0),
            layout: layout.clone(),
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
    ) -> Result<(), rendy_core::hal::device::OutOfMemory> {
        let mut sets = SmallVec::<[_; 32]>::new();

        allocator.allocate(
            device,
            layout.raw(),
            layout.info().ranges(),
            count,
            &mut sets,
        )?;

        assert_eq!(sets.len() as u32, count);

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

    /// Get reference to raw descriptor set resource.
    pub fn raw(&self) -> &B::DescriptorSet {
        self.set.raw()
    }

    /// Get mutable reference to raw descriptor set resource.
    pub unsafe fn raw_mut(&mut self) -> &mut B::DescriptorSet {
        self.set.raw_mut()
    }

    /// Get layout of descriptor set.
    pub fn layout(&mut self) -> &Handle<DescriptorSetLayout<B>> {
        &self.layout
    }
}
