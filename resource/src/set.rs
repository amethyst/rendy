use {
    crate::{descriptor, escape::Handle},
    gfx_hal::{pso::DescriptorSetLayoutBinding, Backend, Device as _},
    relevant::Relevant,
    smallvec::SmallVec,
};

/// Descriptor set layout info.
#[derive(Clone, Debug)]
pub struct Info {
    /// Bindings
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

impl Info {
    pub fn ranges(&self) -> descriptor::DescriptorRanges {
        descriptor::DescriptorRanges::from_bindings(&self.bindings)
    }
}

#[derive(Debug)]
pub struct DescriptorSetLayout<B: Backend> {
    raw: B::DescriptorSetLayout,
    info: Info,
    relevant: Relevant,
}

impl<B> DescriptorSetLayout<B>
where
    B: Backend,
{
    /// Create new descriptor set layout
    pub unsafe fn create(
        device: &B::Device,
        info: Info,
    ) -> Result<Self, gfx_hal::device::OutOfMemory> {
        let raw = device
            .create_descriptor_set_layout(&info.bindings, std::iter::empty::<B::Sampler>())?;

        Ok(DescriptorSetLayout {
            raw,
            info,
            relevant: Relevant,
        })
    }

    pub unsafe fn dispose(self, device: &B::Device) {
        device.destroy_descriptor_set_layout(self.raw);
        self.relevant.dispose();
    }

    pub fn raw(&self) -> &B::DescriptorSetLayout {
        &self.raw
    }

    pub unsafe fn raw_mut(&mut self) -> &mut B::DescriptorSetLayout {
        &mut self.raw
    }

    pub fn info(&self) -> &Info {
        &self.info
    }
}

/// Descriptor set object wrapper.
#[derive(Debug)]
pub struct DescriptorSet<B: Backend> {
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
        device: &B::Device,
        allocator: &mut descriptor::DescriptorAllocator<B>,
        layout: Handle<DescriptorSetLayout<B>>,
    ) -> Result<Self, gfx_hal::device::OutOfMemory> {
        let mut sets = SmallVec::<[_; 1]>::new();

        allocator.allocate(device, layout.raw(), layout.info().ranges(), 1, &mut sets)?;

        assert_eq!(sets.len() as u32, 1);
        Ok(DescriptorSet {
            set: sets.swap_remove(0),
            layout: layout.clone(),
            relevant: Relevant,
        })
    }

    /// Create new descriptor sets.
    pub unsafe fn create_many(
        device: &B::Device,
        allocator: &mut descriptor::DescriptorAllocator<B>,
        layout: Handle<DescriptorSetLayout<B>>,
        count: u32,
        extend: &mut impl Extend<Self>,
    ) -> Result<(), gfx_hal::device::OutOfMemory> {
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
            set,
            layout: layout.clone(),
            relevant: Relevant,
        }));

        Ok(())
    }

    pub unsafe fn dispose(self, allocator: &mut descriptor::DescriptorAllocator<B>) {
        allocator.free(Some(self.set));
        self.relevant.dispose();
    }

    pub fn raw(&self) -> &B::DescriptorSet {
        self.set.raw()
    }

    pub unsafe fn raw_mut(&mut self) -> &mut B::DescriptorSet {
        self.set.raw_mut()
    }

    pub fn layout(&mut self) -> &DescriptorSetLayout<B> {
        &self.layout
    }
}
