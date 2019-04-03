use {
    crate::{
        command::{
            families_from_device, CommandPool, Families, Family, FamilyId, Fence, QueueType, Reset,
        },
        config::{Config, DevicesConfigure, HeapsConfigure, QueuesConfigure},
        descriptor::DescriptorAllocator,
        memory::{self, Heaps, MemoryUsage, Write},
        resource::{
            buffer::{self, Buffer},
            image::{self, Image, ImageView},
            sampler::{Sampler, SamplerCache},
            set::{self, DescriptorSet, DescriptorSetLayout},
            Epochs, Escape, Handle, ResourceTracker,
        },
        upload::{BufferState, ImageState, ImageStateOrLayout, Uploader},
        util::rendy_slow_assert,
        wsi::{Surface, Target},
    },
    gfx_hal::{
        device::*, error::HostExecutionError, format, pso::DescriptorSetLayoutBinding, Adapter,
        Backend, Device, Features, Gpu, Instance, Limits, PhysicalDevice, Surface as GfxSurface,
    },
    smallvec::SmallVec,
    std::{borrow::BorrowMut, cmp::max, mem::ManuallyDrop},
};

static FACTORY_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Debug, derivative::Derivative)]
#[derivative(Default(bound = ""))]
struct ResourceHub<B: Backend> {
    buffers: ResourceTracker<Buffer<B>>,
    images: ResourceTracker<Image<B>>,
    views: ResourceTracker<ImageView<B>>,
    layouts: ResourceTracker<DescriptorSetLayout<B>>,
    sets: ResourceTracker<DescriptorSet<B>>,
    samplers: ResourceTracker<Sampler<B>>,
    samplers_cache: parking_lot::RwLock<SamplerCache<B>>,
}

impl<B> ResourceHub<B>
where
    B: Backend,
{
    unsafe fn cleanup(
        &mut self,
        device: &B::Device,
        heaps: &mut Heaps<B>,
        allocator: &mut DescriptorAllocator<B>,
        next: Epochs,
        complete: Epochs,
    ) {
        self.sets
            .cleanup(|s| s.dispose(allocator), &next, &complete);
        self.views.cleanup(|v| v.dispose(device), &next, &complete);
        self.layouts
            .cleanup(|l| l.dispose(device), &next, &complete);
        self.buffers
            .cleanup(|b| b.dispose(device, heaps), &next, &complete);
        self.images
            .cleanup(|i| i.dispose(device, heaps), &next, &complete);
        self.samplers
            .cleanup(|i| i.dispose(device), &next, &complete);
    }

    unsafe fn dispose(
        mut self,
        device: &B::Device,
        heaps: &mut Heaps<B>,
        allocator: &mut DescriptorAllocator<B>,
    ) {
        drop(self.samplers_cache);
        self.sets.dispose(|s| s.dispose(allocator));
        self.views.dispose(|v| v.dispose(device));
        self.layouts.dispose(|l| l.dispose(device));
        self.buffers.dispose(|b| b.dispose(device, heaps));
        self.images.dispose(|i| i.dispose(device, heaps));
        self.samplers.dispose(|i| i.dispose(device));
    }
}

/// Higher level device interface.
/// Manges memory, resources and queue families.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Factory<B: Backend> {
    descriptor_allocator: ManuallyDrop<parking_lot::Mutex<DescriptorAllocator<B>>>,
    heaps: ManuallyDrop<parking_lot::Mutex<Heaps<B>>>,
    resources: ManuallyDrop<ResourceHub<B>>,
    epochs: Vec<parking_lot::RwLock<Vec<u64>>>,
    uploader: Uploader<B>,
    families_indices: Vec<usize>,
    #[derivative(Debug = "ignore")]
    device: B::Device,
    #[derivative(Debug = "ignore")]
    adapter: Adapter<B>,
    #[derivative(Debug = "ignore")]
    instance: Box<dyn std::any::Any + Send + Sync>,
    id: usize,
}

impl<B> Drop for Factory<B>
where
    B: Backend,
{
    fn drop(&mut self) {
        log::debug!("Dropping factory");
        let _ = self.wait_idle();

        unsafe {
            // Device is idle.
            self.uploader.dispose(&self.device);
            log::trace!("Uploader disposed");
            std::ptr::read(&mut *self.resources).dispose(
                &self.device,
                self.heaps.get_mut(),
                self.descriptor_allocator.get_mut(),
            );

            log::trace!("Resources disposed");
        }

        unsafe {
            std::ptr::read(&mut *self.heaps)
                .into_inner()
                .dispose(&self.device);
            log::trace!("Heaps disposed");
        }

        unsafe {
            std::ptr::read(&mut *self.descriptor_allocator)
                .into_inner()
                .dispose(&self.device);
            log::trace!("Descriptor allocator disposed");
        }

        log::trace!("Factory dropped");
    }
}

impl<B> Factory<B>
where
    B: Backend,
{
    /// Get this factory's unique id
    pub fn id(&self) -> usize {
        self.id
    }

    /// Wait for whole device become idle.
    /// This function is very heavy and
    /// usually used only for teardown.
    pub fn wait_idle(&self) -> Result<(), HostExecutionError> {
        log::debug!("Wait device idle");
        self.device.wait_idle()?;
        log::trace!("Device idle");
        Ok(())
    }

    /// Creates a buffer with the specified properties.
    ///
    /// This function returns relevant value, that it, the value cannot be dropped.
    /// However buffer can be destroyed using [`destroy_relevant_buffer`] function.
    ///
    /// [`destroy_relevant_buffer`]: #method.destroy_relevant_buffer
    pub fn create_relevant_buffer(
        &self,
        info: buffer::Info,
        memory_usage: impl MemoryUsage,
    ) -> Result<Buffer<B>, failure::Error> {
        unsafe { Buffer::create(&self.device, &mut self.heaps.lock(), info, memory_usage) }
    }

    /// Destroy buffer.
    pub unsafe fn destroy_relevant_buffer(&self, buffer: Buffer<B>) {
        buffer.dispose(&self.device, &mut self.heaps.lock());
    }

    /// Creates a buffer with the specified properties.
    ///
    /// This function (unlike [`create_relevant_buffer`]) returns value that can be dropped.
    ///
    /// [`create_relevant_buffer`]: #method.create_relevant_buffer
    pub fn create_buffer(
        &self,
        info: buffer::Info,
        memory_usage: impl MemoryUsage,
    ) -> Result<Escape<Buffer<B>>, failure::Error> {
        let buffer = self.create_relevant_buffer(info, memory_usage)?;
        Ok(self.resources.buffers.escape(buffer))
    }

    /// Creates an image with the specified properties.
    ///
    /// This function returns relevant value, that it, the value cannot be dropped.
    /// However image can be destroyed using [`destroy_relevant_image`] function.
    ///
    /// [`destroy_relevant_image`]: #method.destroy_relevant_image
    pub fn create_relevant_image(
        &self,
        info: image::Info,
        memory_usage: impl MemoryUsage,
    ) -> Result<Image<B>, failure::Error> {
        unsafe { Image::create(&self.device, &mut self.heaps.lock(), info, memory_usage) }
    }

    /// Destroy image.
    pub unsafe fn destroy_relevant_image(&self, image: Image<B>) {
        image.dispose(&self.device, &mut self.heaps.lock());
    }

    /// Creates an image with the specified properties.
    ///
    /// This function (unlike [`create_relevant_image`]) returns value that can be dropped.
    ///
    /// [`create_relevant_image`]: #method.create_relevant_image
    pub fn create_image(
        &self,
        info: image::Info,
        memory_usage: impl MemoryUsage,
    ) -> Result<Escape<Image<B>>, failure::Error> {
        let image = self.create_relevant_image(info, memory_usage)?;
        Ok(self.resources.images.escape(image))
    }

    /// Create an image view with the specified properties
    ///
    /// This function returns relevant value, that it, the value cannot be dropped.
    /// However image can be destroyed using [`destroy_relevant_image_view`] function.
    ///
    /// [`destroy_relevant_image_view`]: #method.destroy_relevant_image_view
    pub fn create_relevant_image_view(
        &self,
        image: Handle<Image<B>>,
        info: image::ViewInfo,
    ) -> Result<ImageView<B>, failure::Error> {
        // TODO: Check image belongs to this factory.
        unsafe { ImageView::create(&self.device, info, image) }
    }

    /// Create an image view with the specified properties
    ///
    /// This function (unlike [`create_relevant_image_view`]) returns value that can be dropped.
    ///
    /// [`create_relevant_image_view`]: #method.create_relevant_image_view
    pub fn create_image_view(
        &self,
        image: Handle<Image<B>>,
        info: image::ViewInfo,
    ) -> Result<Escape<ImageView<B>>, failure::Error> {
        let view = self.create_relevant_image_view(image, info)?;
        Ok(self.resources.views.escape(view))
    }

    /// Destroy image view.
    pub unsafe fn destroy_relevant_image_view(&self, view: ImageView<B>) {
        view.dispose(&self.device);
    }

    /// Create a sampler
    pub fn create_relevant_sampler(
        &self,
        info: gfx_hal::image::SamplerInfo,
    ) -> Result<Sampler<B>, gfx_hal::device::AllocationError> {
        unsafe { Sampler::create(&self.device, info) }
    }

    /// Create a sampler
    pub fn create_sampler(
        &self,
        info: gfx_hal::image::SamplerInfo,
    ) -> Result<Escape<Sampler<B>>, gfx_hal::device::AllocationError> {
        let sampler = self.create_relevant_sampler(info)?;
        Ok(self.resources.samplers.escape(sampler))
    }

    /// Create a sampler
    pub fn get_sampler(
        &self,
        info: gfx_hal::image::SamplerInfo,
    ) -> Result<Handle<Sampler<B>>, gfx_hal::device::AllocationError> {
        let samplers = &self.resources.samplers;
        let device = &self.device;

        SamplerCache::get_with_upgradable_lock(
            self.resources.samplers_cache.upgradable_read(),
            parking_lot::RwLockUpgradableReadGuard::upgrade,
            info.clone(),
            || Ok(samplers.handle(unsafe { Sampler::create(device, info) }?)),
        )
    }

    /// Update buffer bound to host visible memory.vk::AccessFlags.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to or read from
    /// the memory region occupied by this buffer.
    pub unsafe fn upload_visible_buffer<T>(
        &self,
        buffer: &mut Buffer<B>,
        offset: u64,
        content: &[T],
    ) -> Result<(), failure::Error> {
        let content = std::slice::from_raw_parts(
            content.as_ptr() as *const u8,
            content.len() * std::mem::size_of::<T>(),
        );

        let mut mapped = buffer.map(&self.device, offset..offset + content.len() as u64)?;
        mapped
            .write(&self.device, 0..content.len() as u64)?
            .write(content);
        Ok(())
    }

    /// Update buffer content.
    ///
    /// # Safety
    ///
    /// * Buffer must be created by this `Factory`.
    /// * Buffer must not be used by device.
    /// * `state` must match first buffer usage by device after content uploaded.
    pub unsafe fn upload_buffer<T>(
        &self,
        buffer: &Buffer<B>,
        offset: u64,
        content: &[T],
        last: Option<BufferState>,
        next: BufferState,
    ) -> Result<(), failure::Error> {
        let content_size = content.len() as u64 * std::mem::size_of::<T>() as u64;
        let mut staging = self.create_buffer(
            buffer::Info {
                size: content_size,
                usage: gfx_hal::buffer::Usage::TRANSFER_SRC,
            },
            memory::Upload,
        )?;

        self.upload_visible_buffer(&mut staging, 0, content)?;

        self.uploader
            .upload_buffer(&self.device, buffer, offset, staging, last, next)
    }

    /// Upload image.
    ///
    /// # Safety
    ///
    /// * Image must be created by this `Factory`.
    /// * Image must not be used by device.
    /// * `state` must match first image usage by device after content uploaded.
    pub unsafe fn upload_image<T>(
        &self,
        image: &Image<B>,
        data_width: u32,
        data_height: u32,
        image_layers: image::SubresourceLayers,
        image_offset: image::Offset,
        image_extent: image::Extent,
        content: &[T],
        last: impl Into<ImageStateOrLayout>,
        next: ImageState,
    ) -> Result<(), failure::Error> {
        assert_eq!(image.format().surface_desc().aspects, image_layers.aspects);
        assert!(image_layers.layers.start <= image_layers.layers.end);
        assert!(image_layers.layers.end <= image.kind().num_layers());
        assert!(image_layers.level <= image.info().levels);

        let content_size = content.len() as u64 * std::mem::size_of::<T>() as u64;
        let format_desc = image.format().surface_desc();
        let texels_count = (image_extent.width / format_desc.dim.0 as u32) as u64
            * (image_extent.height / format_desc.dim.1 as u32) as u64
            * image_extent.depth as u64;
        let total_bytes = (format_desc.bits as u64 / 8) * texels_count;
        assert_eq!(
            total_bytes, content_size,
            "Size of must match size of the image region"
        );

        let mut staging = self.create_buffer(
            buffer::Info {
                size: content_size,
                usage: gfx_hal::buffer::Usage::TRANSFER_SRC,
            },
            memory::Upload,
        )?;

        self.upload_visible_buffer(&mut staging, 0, content)?;

        self.uploader.upload_image(
            &self.device,
            image,
            data_width,
            data_height,
            image_layers,
            image_offset,
            image_extent,
            staging,
            last.into(),
            next,
        )
    }

    /// Create rendering surface from window.
    pub fn create_surface(&mut self, window: std::sync::Arc<winit::Window>) -> Surface<B> {
        Surface::new(&*self.instance, window, self.id)
    }

    /// Get compatibility of Surface
    ///
    /// ## Panics
    /// - Panics if `no-slow-safety-checks` feature is disabled and
    /// `surface` was not created by this `Factory`
    pub fn get_surface_compatibility(
        &self,
        surface: &Surface<B>,
    ) -> (
        gfx_hal::window::SurfaceCapabilities,
        Option<Vec<gfx_hal::format::Format>>,
        Vec<gfx_hal::PresentMode>,
        Vec<gfx_hal::CompositeAlpha>,
    ) {
        rendy_slow_assert!(surface.factory_id() == self.id);
        unsafe { surface.compatibility(&self.adapter.physical_device) }
    }

    /// Get surface format.
    ///
    /// ## Panics
    /// - Panics if `no-slow-safety-checks` feature is disabled and
    /// `surface` was not created by this `Factory`
    pub fn get_surface_format(&self, surface: &Surface<B>) -> format::Format {
        rendy_slow_assert!(surface.factory_id() == self.id);
        unsafe { surface.format(&self.adapter.physical_device) }
    }

    /// Destroy surface returning underlying window back to the caller.
    ///
    /// ## Panics
    /// - Panics if `no-slow-safety-checks` feature is disabled and
    /// `surface` was not created by this `Factory`
    pub unsafe fn destroy_surface(&mut self, surface: Surface<B>) {
        rendy_slow_assert!(surface.factory_id() == self.id);
        drop(surface);
    }

    /// Create target out of rendering surface. The compatibility of
    /// the surface with the queue family which will present to
    /// this target must have *already* been checked using
    /// `Factory::surface_support`.
    ///
    /// ## Panics
    /// - Panics if `no-slow-safety-checks` feature is disabled and
    /// `surface` was not created by this `Factory`
    pub fn create_target(
        &self,
        surface: Surface<B>,
        image_count: u32,
        present_mode: gfx_hal::PresentMode,
        usage: gfx_hal::image::Usage,
    ) -> Result<Target<B>, failure::Error> {
        rendy_slow_assert!(surface.factory_id() == self.id);
        unsafe {
            surface.into_target(
                &self.adapter.physical_device,
                &self.device,
                image_count,
                present_mode,
                usage,
            )
        }
    }

    /// Destroy target returning underlying surface back to the caller.
    pub unsafe fn destroy_target(&self, target: Target<B>) -> Surface<B> {
        target.dispose(&self.device)
    }

    /// Get surface support for family.
    pub fn surface_support(&self, family: FamilyId, surface: &B::Surface) -> bool {
        surface.supports_queue_family(&self.adapter.queue_families[family.0])
    }

    /// Get device.
    pub fn device(&self) -> &impl Device<B> {
        &self.device
    }

    /// Get physical device.
    pub fn physical(&self) -> &B::PhysicalDevice {
        &self.adapter.physical_device
    }

    /// Create new semaphore
    pub fn create_semaphore(&self) -> Result<B::Semaphore, OutOfMemory> {
        self.device.create_semaphore()
    }

    /// Destroy semaphore
    pub unsafe fn destroy_semaphore(&self, semaphore: B::Semaphore) {
        self.device.destroy_semaphore(semaphore);
    }

    /// Create new fence
    pub fn create_fence(&self, signaled: bool) -> Result<Fence<B>, OutOfMemory> {
        Fence::new(&self.device, signaled)
    }

    /// Wait for the fence become signeled.
    pub unsafe fn reset_fence(&self, fence: &mut Fence<B>) -> Result<(), OutOfMemory> {
        fence.reset(&self.device)
    }

    /// Wait for the fence become signeled.
    pub fn reset_fences<'a>(
        &self,
        fences: impl IntoIterator<Item = &'a mut (impl BorrowMut<Fence<B>> + 'a)>,
    ) -> Result<(), OutOfMemory> {
        let fences = fences
            .into_iter()
            .map(|f| {
                let f = f.borrow_mut();
                assert!(f.is_signaled());
                f
            })
            .collect::<SmallVec<[_; 32]>>();
        unsafe { self.device.reset_fences(fences.iter().map(|f| f.raw())) }?;
        fences.into_iter().for_each(|f| unsafe {
            /*all reset*/
            f.mark_reset()
        });
        Ok(())
    }

    /// Wait for the fence become signeled.
    pub unsafe fn wait_for_fence(
        &self,
        fence: &mut Fence<B>,
        timeout_ns: u64,
    ) -> Result<bool, OomOrDeviceLost> {
        if let Some(fence_epoch) = fence.wait_signaled(&self.device, timeout_ns)? {
            // Now we can update epochs counter.
            let family_index = self.families_indices[fence_epoch.queue.family().0];
            let mut lock = self.epochs[family_index].write();
            let epoch = &mut lock[fence_epoch.queue.index()];
            *epoch = max(*epoch, fence_epoch.epoch);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Wait for the fences become signeled.
    pub fn wait_for_fences<'a>(
        &self,
        fences: impl IntoIterator<Item = &'a mut (impl BorrowMut<Fence<B>> + 'a)>,
        wait_for: WaitFor,
        timeout_ns: u64,
    ) -> Result<bool, OomOrDeviceLost> {
        let fences = fences
            .into_iter()
            .map(|f| f.borrow_mut())
            .collect::<SmallVec<[_; 32]>>();

        unsafe {
            if !self
                .device
                .wait_for_fences(fences.iter().map(|f| f.raw()), wait_for, timeout_ns)?
            {
                return Ok(false);
            }
        }

        let mut epoch_locks = SmallVec::<[_; 32]>::new();
        for fence in &fences {
            let family_id = fence.epoch().queue.family();
            while family_id.0 >= epoch_locks.len() {
                epoch_locks.push(None);
            }
        }

        match wait_for {
            WaitFor::Any => {
                for fence in fences {
                    if unsafe { self.device.get_fence_status(fence.raw()) }? {
                        let epoch = unsafe {
                            /*status checked*/
                            fence.mark_signaled()
                        };
                        let family_id = epoch.queue.family();
                        let family_index = *self
                            .families_indices
                            .get(family_id.0)
                            .expect("Valid family id expected");
                        let lock = epoch_locks[family_id.0]
                            .get_or_insert_with(|| self.epochs[family_index].write());
                        let queue_epoch = &mut lock[epoch.queue.index()];
                        *queue_epoch = max(*queue_epoch, epoch.epoch);
                    }
                }
            }
            WaitFor::All => {
                for fence in fences {
                    let epoch = unsafe {
                        /*all fences signaled*/
                        fence.mark_signaled()
                    };
                    let family_id = epoch.queue.family();
                    let family_index = *self
                        .families_indices
                        .get(family_id.0)
                        .expect("Valid family id expected");
                    let lock = epoch_locks[family_id.0]
                        .get_or_insert_with(|| self.epochs[family_index].write());
                    let queue_epoch = &mut lock[epoch.queue.index()];
                    *queue_epoch = max(*queue_epoch, epoch.epoch);
                }
            }
        }
        Ok(true)
    }

    /// Destroy fence.
    pub fn destroy_fence(&self, fence: Fence<B>) {
        unsafe { self.device.destroy_fence(fence.into_inner()) }
    }

    /// Create new command pool for specified family.
    pub fn create_command_pool<R>(
        &self,
        family: &Family<B>,
    ) -> Result<CommandPool<B, QueueType, R>, failure::Error>
    where
        R: Reset,
    {
        family.create_pool(&self.device).map_err(Into::into)
    }

    /// Create new command pool for specified family.
    pub unsafe fn destroy_command_pool<C, R>(&self, pool: CommandPool<B, C, R>)
    where
        R: Reset,
    {
        pool.dispose(&self.device);
    }

    fn next_epochs(&mut self, families: &Families<B>) -> Epochs {
        Epochs {
            values: families
                .as_slice()
                .iter()
                .map(|f| f.as_slice().iter().map(|q| q.next_epoch()).collect())
                .collect(),
        }
    }

    fn complete_epochs(&mut self) -> Epochs {
        Epochs {
            values: self
                .epochs
                .iter_mut()
                .map(|l| l.get_mut().iter().cloned().collect())
                .collect(),
        }
    }

    /// Cleanup unused resources
    pub fn cleanup(&mut self, families: &Families<B>) {
        let next = self.next_epochs(families);
        let complete = self.complete_epochs();
        unsafe {
            self.uploader.cleanup(&self.device);
            self.resources.cleanup(
                &self.device,
                self.heaps.get_mut(),
                self.descriptor_allocator.get_mut(),
                next,
                complete,
            );

            self.descriptor_allocator.get_mut().cleanup(&self.device);
        }
    }

    /// Flush uploads
    pub fn flush_uploads(&mut self, families: &mut Families<B>) {
        unsafe { self.uploader.flush(families) }
    }

    /// Flush uploads and cleanup unused resources.
    pub fn maintain(&mut self, families: &mut Families<B>) {
        self.flush_uploads(families);
        self.cleanup(families);
    }

    /// Create descriptor set layout with specified bindings.
    pub fn create_relevant_descriptor_set_layout(
        &self,
        bindings: Vec<DescriptorSetLayoutBinding>,
    ) -> Result<DescriptorSetLayout<B>, OutOfMemory> {
        unsafe { DescriptorSetLayout::create(&self.device, set::Info { bindings }) }
    }

    /// Create descriptor set layout with specified bindings.
    pub fn create_descriptor_set_layout(
        &self,
        bindings: Vec<DescriptorSetLayoutBinding>,
    ) -> Result<Escape<DescriptorSetLayout<B>>, OutOfMemory> {
        let layout = self.create_relevant_descriptor_set_layout(bindings)?;
        Ok(self.resources.layouts.escape(layout))
    }

    /// Create descriptor sets with specified layout.
    pub fn create_relevant_descriptor_set(
        &self,
        layout: Handle<DescriptorSetLayout<B>>,
    ) -> Result<DescriptorSet<B>, OutOfMemory> {
        // TODO: Check `layout` belongs to this factory.
        unsafe {
            DescriptorSet::create(&self.device, &mut self.descriptor_allocator.lock(), layout)
        }
    }

    /// Create descriptor sets with specified layout.
    pub fn create_descriptor_set(
        &self,
        layout: Handle<DescriptorSetLayout<B>>,
    ) -> Result<Escape<DescriptorSet<B>>, OutOfMemory> {
        let set = self.create_relevant_descriptor_set(layout)?;
        Ok(self.resources.sets.escape(set))
    }

    /// Create descriptor sets with specified layout.
    ///
    /// # Safety
    ///
    /// `layout` must be created by this `Factory`.
    ///
    pub fn create_descriptor_sets<T>(
        &self,
        layout: Handle<DescriptorSetLayout<B>>,
        count: u32,
    ) -> Result<T, OutOfMemory>
    where
        T: std::iter::FromIterator<Escape<DescriptorSet<B>>>,
    {
        let mut result = SmallVec::<[_; 32]>::new();
        unsafe {
            DescriptorSet::create_many(
                &self.device,
                &mut self.descriptor_allocator.lock(),
                layout,
                count,
                &mut result,
            )
        }?;

        Ok(result
            .into_iter()
            .map(|set| self.resources.sets.escape(set))
            .collect())
    }
}

#[doc(hidden)]
impl<B> std::ops::Deref for Factory<B>
where
    B: Backend,
{
    type Target = B::Device;

    fn deref(&self) -> &B::Device {
        &self.device
    }
}

macro_rules! init_for_backend {
    (match $target:ident, $config:ident $(| $backend:ident @ $feature:meta)+) => {{
        #[allow(non_camel_case_types)]
        enum _B {$(
            $backend,
        )+}

        for b in [$(_B::$backend),+].iter() {
            match b {$(
                #[$feature]
                _B::$backend => {
                    if std::any::TypeId::of::<$backend::Backend>() == std::any::TypeId::of::<$target>() {
                        let instance = $backend::Instance::create("Rendy", 1);

                        let (factory, families) = init_with_instance(instance, $config)?;

                        let factory: Box<dyn std::any::Any> = Box::new(factory);
                        let families: Box<dyn std::any::Any> = Box::new(families);
                        return Ok((
                            *factory.downcast::<Factory<$target>>().unwrap(),
                            *families.downcast::<Families<$target>>().unwrap(),
                        ));
                    }
                })+
                _ => continue,
            }
        }
        panic!("
            Undefined backend requested.
            Make sure feature for required backend is enabled.
            Try to add `--features=vulkan` or if on macos `--features=metal`.
        ")
    }};

    ($target:ident, $config:ident) => {{
        init_for_backend!(match $target, $config
            | gfx_backend_empty @ cfg(feature = "empty")
            | gfx_backend_dx12 @ cfg(feature = "dx12")
            | gfx_backend_metal @ cfg(feature = "metal")
            | gfx_backend_vulkan @ cfg(feature = "vulkan")
        );
    }};
}

/// Initialize `Factory` and Queue `Families` associated with Device.
#[allow(unused_variables)]
pub fn init<B>(
    config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
) -> Result<(Factory<B>, Families<B>), failure::Error>
where
    B: gfx_hal::Backend,
{
    log::debug!("Creating factory");
    init_for_backend!(B, config)
}

/// Initialize `Factory` and Queue `Families` associated with Device
/// using existing `Instance`.
pub fn init_with_instance<B>(
    instance: impl Instance<Backend = B>,
    config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
) -> Result<(Factory<B>, Families<B>), failure::Error>
where
    B: gfx_hal::Backend,
{
    #[cfg(not(feature = "no-slow-safety-checks"))]
    log::warn!("Slow safety checks are enabled! Disable them in production by enabling the 'no-slow-safety-checks' feature!");
    let mut adapters = instance.enumerate_adapters();

    if adapters.is_empty() {
        failure::bail!("No physical devices found");
    }

    log::info!(
        "Physical devices:\n{:#?}",
        adapters
            .iter()
            .map(|adapter| &adapter.info)
            .collect::<SmallVec<[_; 32]>>()
    );

    let picked = config.devices.pick(&adapters);
    if picked >= adapters.len() {
        panic!("Physical device pick config returned index out of bound");
    }
    let adapter = adapters.swap_remove(picked);

    #[derive(Debug)]
    struct PhysicalDeviceInfo<'a> {
        name: &'a str,
        features: Features,
        limits: Limits,
    }

    log::info!(
        "Physical device picked: {:#?}",
        PhysicalDeviceInfo {
            name: &adapter.info.name,
            features: adapter.physical_device.features(),
            limits: adapter.physical_device.limits(),
        }
    );

    let (id, device, families) = {
        let families = config
            .queues
            .configure(&adapter.queue_families)
            .into_iter()
            .collect::<SmallVec<[_; 16]>>();
        let (create_queues, get_queues): (SmallVec<[_; 32]>, SmallVec<[_; 32]>) = families
            .iter()
            .map(|(index, priorities)| {
                (
                    (&adapter.queue_families[index.0], priorities.as_ref()),
                    (*index, priorities.as_ref().len()),
                )
            })
            .unzip();

        log::info!("Queues: {:#?}", get_queues);

        let Gpu { device, mut queues } = unsafe { adapter.physical_device.open(&create_queues) }?;

        let id = FACTORY_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let families =
            unsafe { families_from_device(&mut queues, get_queues, &adapter.queue_families) };
        (id, device, families)
    };

    let (types, heaps) = config
        .heaps
        .configure(&adapter.physical_device.memory_properties());
    let heaps = heaps.into_iter().collect::<SmallVec<[_; 16]>>();
    let types = types.into_iter().collect::<SmallVec<[_; 32]>>();

    log::info!("Heaps: {:#?}\nTypes: {:#?}", heaps, types);

    let heaps = unsafe { Heaps::new(types, heaps) };

    let epochs = families
        .as_slice()
        .iter()
        .map(|f| parking_lot::RwLock::new(vec![0; f.as_slice().len()]))
        .collect();

    let factory = Factory {
        descriptor_allocator: ManuallyDrop::new(
            parking_lot::Mutex::new(DescriptorAllocator::new()),
        ),
        heaps: ManuallyDrop::new(parking_lot::Mutex::new(heaps)),
        resources: ManuallyDrop::new(ResourceHub::default()),
        uploader: unsafe { Uploader::new(&device, &families) }?,
        families_indices: families.indices().into(),
        epochs,
        device,
        adapter,
        instance: Box::new(instance),
        id,
    };

    Ok((factory, families))
}
