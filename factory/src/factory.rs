use {
    crate::{
        blitter::Blitter,
        command::{
            families_from_device, CommandPool, Families, Family, FamilyId, Fence, QueueType, Reset,
        },
        config::{Config, DevicesConfigure, HeapsConfigure, QueuesConfigure},
        descriptor::DescriptorAllocator,
        memory::{self, Heaps, MemoryUsage, TotalMemoryUtilization, Write},
        resource::*,
        upload::{BufferState, ImageState, ImageStateOrLayout, Uploader},
        core::{
            rendy_with_slow_safety_checks, with_winit, Device, DeviceId, Instance, InstanceId,
            hal::{
                self,
                buffer, device::{Device as _, *}, error::HostExecutionError, format, image,
                pso::DescriptorSetLayoutBinding, window::Extent2D, adapter::{Adapter, PhysicalDevice as _, Gpu}, Backend, Features,
                Limits,
            },
        },
        wsi::{Surface, Swapchain},
    },
    smallvec::SmallVec,
    std::{borrow::BorrowMut, cmp::max, mem::ManuallyDrop},
    thread_profiler::profile_scope,
};

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
        device: &Device<B>,
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
        device: &Device<B>,
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
    blitter: Blitter<B>,
    families_indices: Vec<usize>,
    #[derivative(Debug = "ignore")]
    device: Device<B>,
    #[derivative(Debug = "ignore")]
    adapter: Adapter<B>,
    #[derivative(Debug = "ignore")]
    instance: Instance<B>,
}

#[allow(unused)]
fn factory_is_send_sync<B: Backend>() {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<Factory<B>>();
}

impl<B> Drop for Factory<B>
where
    B: Backend,
{
    fn drop(&mut self) {
        log::debug!("Dropping factory");
        match self.wait_idle() {
            Err(HostExecutionError::DeviceLost) | Ok(()) => (),
            Err(err) => panic!("{}", err),
        }

        unsafe {
            // Device is idle.
            self.uploader.dispose(&self.device);
            log::trace!("Uploader disposed");
            self.blitter.dispose(&self.device);
            log::trace!("Blitter disposed");
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
    /// Get instance id of the factory.
    pub fn instance_id(&self) -> InstanceId {
        self.instance.id()
    }

    /// Wait for whole device become idle.
    /// This function is very heavy and
    /// usually used only for teardown.
    pub fn wait_idle(&self) -> Result<(), HostExecutionError> {
        profile_scope!("wait_idle");

        log::debug!("Wait device idle");
        self.device.wait_idle()?;
        log::trace!("Device idle");
        Ok(())
    }

    /// Creates a buffer with the specified properties.
    ///
    /// This function returns relevant value, that is, the value cannot be dropped.
    /// However buffer can be destroyed using [`destroy_relevant_buffer`] function.
    ///
    /// [`destroy_relevant_buffer`]: #method.destroy_relevant_buffer
    pub fn create_relevant_buffer(
        &self,
        info: BufferInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Buffer<B>, failure::Error> {
        profile_scope!("create_relevant_buffer");

        unsafe { Buffer::create(&self.device, &mut self.heaps.lock(), info, memory_usage) }
    }

    /// Destroy buffer.
    /// If buffer was created using [`create_buffer`] it must be unescaped first.
    /// If buffer was shaderd unescaping may fail due to other owners existing.
    /// In any case unescaping and destroying manually can slightly increase performance.
    ///
    /// # Safety
    ///
    /// Buffer must not be used by any pending commands or referenced anywhere.
    ///
    /// [`create_buffer`]: #method.create_buffer
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
        info: BufferInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Escape<Buffer<B>>, failure::Error> {
        let buffer = self.create_relevant_buffer(info, memory_usage)?;
        Ok(self.resources.buffers.escape(buffer))
    }

    /// Creates an image with the specified properties.
    ///
    /// This function returns relevant value, that is, the value cannot be dropped.
    /// However image can be destroyed using [`destroy_relevant_image`] function.
    ///
    /// [`destroy_relevant_image`]: #method.destroy_relevant_image
    pub fn create_relevant_image(
        &self,
        info: ImageInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Image<B>, failure::Error> {
        profile_scope!("create_relevant_image");

        unsafe { Image::create(&self.device, &mut self.heaps.lock(), info, memory_usage) }
    }

    /// Destroy image.
    /// If image was created using [`create_image`] it must be unescaped first.
    /// If image was shaderd unescaping may fail due to other owners existing.
    /// In any case unescaping and destroying manually can slightly increase performance.
    ///
    /// # Safety
    ///
    /// Image must not be used by any pending commands or referenced anywhere.
    ///
    /// [`create_image`]: #method.create_image
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
        info: ImageInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<Escape<Image<B>>, failure::Error> {
        let image = self.create_relevant_image(info, memory_usage)?;
        Ok(self.resources.images.escape(image))
    }

    /// Fetch image format details for a particular `ImageInfo`.
    pub fn image_format_properties(&self, info: ImageInfo) -> Option<FormatProperties> {
        self.physical().image_format_properties(
            info.format,
            match info.kind {
                Kind::D1(_, _) => 1,
                Kind::D2(_, _, _, _) => 2,
                Kind::D3(_, _, _) => 3,
            },
            info.tiling,
            info.usage,
            info.view_caps,
        )
    }

    /// Create an image view with the specified properties
    ///
    /// This function returns relevant value, that is, the value cannot be dropped.
    /// However image view can be destroyed using [`destroy_relevant_image_view`] function.
    ///
    /// [`destroy_relevant_image_view`]: #method.destroy_relevant_image_view
    pub fn create_relevant_image_view(
        &self,
        image: Handle<Image<B>>,
        info: ImageViewInfo,
    ) -> Result<ImageView<B>, failure::Error> {
        ImageView::create(&self.device, info, image)
    }

    /// Destroy image view.
    /// If image view was created using [`create_image_view`] it must be unescaped first.
    /// If image view was shaderd unescaping may fail due to other owners existing.
    /// In any case unescaping and destroying manually can slightly increase performance.
    ///
    /// # Safety
    ///
    /// Image view must not be used by any pending commands or referenced anywhere.
    ///
    /// [`create_image_view`]: #method.create_image_view
    pub unsafe fn destroy_relevant_image_view(&self, view: ImageView<B>) {
        view.dispose(&self.device);
    }

    /// Create an image view with the specified properties
    ///
    /// This function (unlike [`create_relevant_image_view`]) returns value that can be dropped.
    ///
    /// [`create_relevant_image_view`]: #method.create_relevant_image_view
    pub fn create_image_view(
        &self,
        image: Handle<Image<B>>,
        info: ImageViewInfo,
    ) -> Result<Escape<ImageView<B>>, failure::Error> {
        let view = self.create_relevant_image_view(image, info)?;
        Ok(self.resources.views.escape(view))
    }

    /// Create an sampler with the specified properties
    ///
    /// This function returns relevant value, that is, the value cannot be dropped.
    /// However sampler can be destroyed using [`destroy_relevant_sampler`] function.
    ///
    /// [`destroy_relevant_sampler`]: #method.destroy_relevant_sampler
    pub fn create_relevant_sampler(
        &self,
        info: SamplerInfo,
    ) -> Result<Sampler<B>, AllocationError> {
        Sampler::create(&self.device, info)
    }

    /// Destroy sampler.
    /// If sampler was created using [`create_sampler`] it must be unescaped first.
    /// If sampler was shaderd unescaping may fail due to other owners existing.
    /// In any case unescaping and destroying manually can slightly increase performance.
    /// If sampler was acquired using [`get_sampler`] unescaping will most probably fail
    /// due to factory holding handler's copy in cache.
    ///
    /// # Safety
    ///
    /// Sampler view must not be used by any pending commands or referenced anywhere.
    ///
    /// [`create_sampler`]: #method.create_sampler
    /// [`get_sampler`]: #method.get_sampler
    pub unsafe fn destroy_relevant_sampler(&self, sampler: Sampler<B>) {
        sampler.dispose(&self.device);
    }

    /// Creates a sampler with the specified properties.
    ///
    /// This function (unlike [`create_relevant_sampler`]) returns value that can be dropped.
    ///
    /// [`create_relevant_sampler`]: #method.create_relevant_sampler
    pub fn create_sampler(&self, info: SamplerInfo) -> Result<Escape<Sampler<B>>, AllocationError> {
        let sampler = self.create_relevant_sampler(info)?;
        Ok(self.resources.samplers.escape(sampler))
    }

    /// Get cached sampler or create new one.
    /// User should prefer this function to [`create_sampler`] and [`create_relevant_sampler`]
    /// because usually only few sampler configuration is required.
    ///
    /// [`create_sampler`]: #method.create_sampler
    /// [`create_relevant_sampler`]: #method.create_relevant_sampler
    pub fn get_sampler(&self, info: SamplerInfo) -> Result<Handle<Sampler<B>>, AllocationError> {
        let samplers = &self.resources.samplers;
        let device = &self.device;

        SamplerCache::get_with_upgradable_lock(
            self.resources.samplers_cache.upgradable_read(),
            parking_lot::RwLockUpgradableReadGuard::upgrade,
            info.clone(),
            || Ok(samplers.handle(Sampler::create(device, info)?)),
        )
    }

    /// Update content of the buffer bound to host visible memory.
    /// This function (unlike [`upload_buffer`]) update content immediatelly.
    ///
    /// Buffers allocated from host-invisible memory types cannot be
    /// updated via this function.
    ///
    /// Updated content will be automatically made visible to device operations
    /// that will be submitted later.
    ///
    /// # Panics
    ///
    /// Panics if buffer size is less than `offset` + size of `content`.
    ///
    /// # Safety
    ///
    /// Caller must ensure that device doesn't use memory region that being updated.
    ///
    /// [`upload_buffer`]: #method.upload_buffer
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

    /// Update buffer range content with provided data.
    ///
    /// Update operation will actually be submitted to the graphics device queue
    /// upon next [`flush_uploads`] or [`maintain`] call to this `Factory`, and
    /// is guaranteed to take place after all previous operations that have been
    /// submitted to the same graphics queue on this `Factory` since last
    /// [`flush_uploads`] or [`maintain`] call
    ///
    /// Note that buffer range will receive `content` as raw bytes.
    /// And interpretation will depend solely on device operation.
    /// Slice of generic type is allowed for convenience.
    /// It usually should be POD struct of numeric values or other POD structs.
    ///
    /// `#[repr(C)]` can be used to guarantee defined memory layout of struct fields.
    ///
    /// # Safety
    ///
    /// If buffer is used by device then `last` state must match the last usage state of the buffer
    /// before updating happen.
    /// In order to guarantee that updated content will be made visible to next device operation
    /// that reads content of the buffer range the `next` must match buffer usage state in that operation.
    pub unsafe fn upload_buffer<T>(
        &self,
        buffer: &Buffer<B>,
        offset: u64,
        content: &[T],
        last: Option<BufferState>,
        next: BufferState,
    ) -> Result<(), failure::Error> {
        assert!(buffer.info().usage.contains(buffer::Usage::TRANSFER_DST));

        let content_size = content.len() as u64 * std::mem::size_of::<T>() as u64;
        let mut staging = self.create_buffer(
            BufferInfo {
                size: content_size,
                usage: buffer::Usage::TRANSFER_SRC,
            },
            memory::Upload,
        )?;

        self.upload_visible_buffer(&mut staging, 0, content)?;

        self.uploader
            .upload_buffer(&self.device, buffer, offset, staging, last, next)
    }

    /// Update buffer content with provided staging buffer.
    ///
    /// Update operation will actually be submitted to the graphics device queue
    /// upon next [`flush_uploads`] or [`maintain`] call to this `Factory`, and
    /// is guaranteed to take place after all previous operations that have been
    /// submitted to the same graphics queue on this `Factory` since last
    /// [`flush_uploads`] or [`maintain`] call
    ///
    /// # Safety
    ///
    /// If buffer is used by device then `last` state must match the last usage state of the buffer
    /// before updating happen.
    /// In order to guarantee that updated content will be made visible to next device operation
    /// that reads content of the buffer range the `next` must match buffer usage state in that operation.
    pub unsafe fn upload_from_staging_buffer(
        &self,
        buffer: &Buffer<B>,
        offset: u64,
        staging: Escape<Buffer<B>>,
        last: Option<BufferState>,
        next: BufferState,
    ) -> Result<(), failure::Error> {
        assert!(buffer.info().usage.contains(buffer::Usage::TRANSFER_DST));
        assert!(staging.info().usage.contains(buffer::Usage::TRANSFER_SRC));
        self.uploader
            .upload_buffer(&self.device, buffer, offset, staging, last, next)
    }

    /// Update image layers content with provided data.
    /// Transition part of image from one state to another.
    ///
    /// Update operation will actually be submitted to the graphics device queue
    /// upon next [`flush_uploads`] or [`maintain`] call to this `Factory`, and
    /// is guaranteed to take place after all previous operations that have been
    /// submitted to the same graphics queue on this `Factory` since last
    /// [`flush_uploads`] or [`maintain`] call
    ///
    /// # Safety
    ///
    /// Image must be created by this `Factory`.
    /// If image is used by device then `last` state must match the last usage state of the image
    /// before transition.
    pub unsafe fn transition_image(
        &self,
        image: Handle<Image<B>>,
        image_range: SubresourceRange,
        last: impl Into<ImageStateOrLayout>,
        next: ImageState,
    ) {
        self.uploader
            .transition_image(image, image_range, last.into(), next);
    }

    /// Update image layers content with provided data.
    ///
    /// Update operation will actually be submitted to the graphics device queue
    /// upon next [`flush_uploads`] or [`maintain`] call to this `Factory`, and
    /// is guaranteed to take place after all previous operations that have been
    /// submitted to the same graphics queue on this `Factory` since last
    /// [`flush_uploads`] or [`maintain`] call
    ///
    /// Note that image layers will receive `content` as raw bytes.
    /// And interpretation will depend solely on device operation.
    /// Slice of generic type is allowed for convenience.
    /// It usually should be compatible type of pixel or channel.
    /// For example `&[[u8; 4]]` or `&[u8]` for `Rgba8Unorm` format.
    ///
    /// # Safety
    ///
    /// Image must be created by this `Factory`.
    /// If image is used by device then `last` state must match the last usage state of the image
    /// before updating happen.
    /// In order to guarantee that updated content will be made visible to next device operation
    /// that reads content of the image layers the `next` must match image usage state in that operation.
    pub unsafe fn upload_image<T>(
        &self,
        image: Handle<Image<B>>,
        data_width: u32,
        data_height: u32,
        image_layers: SubresourceLayers,
        image_offset: image::Offset,
        image_extent: Extent,
        content: &[T],
        last: impl Into<ImageStateOrLayout>,
        next: ImageState,
    ) -> Result<(), failure::Error> {
        assert!(image.info().usage.contains(image::Usage::TRANSFER_DST));
        assert_eq!(image.format().surface_desc().aspects, image_layers.aspects);
        assert!(image_layers.layers.start <= image_layers.layers.end);
        assert!(image_layers.layers.end <= image.kind().num_layers());
        assert!(image_layers.level <= image.info().levels);

        let content_size = content.len() as u64 * std::mem::size_of::<T>() as u64;
        let format_desc = image.format().surface_desc();
        let texels_count = (image_extent.width / format_desc.dim.0 as u32) as u64
            * (image_extent.height / format_desc.dim.1 as u32) as u64
            * image_extent.depth as u64
            * (image_layers.layers.end - image_layers.layers.start) as u64;
        let total_bytes = (format_desc.bits as u64 / 8) * texels_count;
        assert_eq!(
            total_bytes, content_size,
            "Size of must match size of the image region"
        );

        let mut staging = self.create_buffer(
            BufferInfo {
                size: content_size,
                usage: buffer::Usage::TRANSFER_SRC,
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

    /// Get blitter instance
    pub fn blitter(&self) -> &Blitter<B> {
        &self.blitter
    }

    /// Destroy surface returning underlying window back to the caller.
    ///
    /// # Panics
    ///
    /// Panics if `surface` was not created by this `Factory`
    pub fn destroy_surface(&mut self, surface: Surface<B>) {
        surface.assert_instance_owner(&self.instance);
        drop(surface);
    }

    /// Create rendering surface from window.
    ///
    /// # Safety
    ///
    /// Closure must return surface object created from raw instance provided as closure argument.
    pub unsafe fn create_surface_with<T>(&mut self, f: impl FnOnce(&T) -> B::Surface) -> Surface<B>
    where
        T: hal::Instance<Backend = B>,
    {
        profile_scope!("create_surface");
        Surface::create(&self.instance, f)
    }

    /// Get compatibility of Surface
    ///
    /// # Panics
    ///
    /// Panics if `surface` was not created by this `Factory`
    pub fn get_surface_compatibility(
        &self,
        surface: &Surface<B>,
    ) -> (
        hal::window::SurfaceCapabilities,
        Option<Vec<hal::format::Format>>,
        Vec<hal::window::PresentMode>,
    ) {
        profile_scope!("get_surface_compatibility");

        surface.assert_instance_owner(&self.instance);
        unsafe { surface.compatibility(&self.adapter.physical_device) }
    }

    /// Get surface format.
    ///
    /// # Panics
    ///
    /// Panics if `surface` was not created by this `Factory`
    pub fn get_surface_format(&self, surface: &Surface<B>) -> format::Format {
        profile_scope!("get_surface_format");

        surface.assert_instance_owner(&self.instance);
        unsafe { surface.format(&self.adapter.physical_device) }
    }

    /// Create swapchain out of rendering surface.
    ///
    /// The compatibility of the surface with the queue family which will present to
    /// this swapchain must have *already* been checked using `Factory::surface_support`.
    ///
    /// # Panics
    ///
    /// Panics if `surface` was not created by this `Factory`.
    pub fn create_swapchain(
        &self,
        surface: Surface<B>,
        extent: Extent2D,
        image_count: Option<u32>,
        present_mode: hal::window::PresentMode,
        usage: image::Usage,
    ) -> Result<Swapchain<B>, failure::Error> {
        profile_scope!("create_swapchain");

        unsafe {
            surface.into_swapchain(
                &self.adapter.physical_device,
                &self.device,
                extent,
                image_count,
                present_mode,
                usage,
            )
        }
    }

    /// Destroy swapchain returning underlying surface back to the caller.
    ///
    /// # Safety
    ///
    /// Swapchain images must not be with_winit, used by pending commands or referenced anywhere.
    pub unsafe fn destroy_swapchain(&self, swapchain: Swapchain<B>) -> Surface<B> {
        swapchain.dispose(&self.device)
    }

    /// Check if queue family supports presentation to the specified surface.
    pub fn surface_support(&self, family: FamilyId, surface: &Surface<B>) -> bool {
        surface.assert_instance_owner(&self.instance);
        unsafe {
            surface.support(&self.adapter.queue_families[family.index])
        }
    }

    /// Get raw device.
    pub fn device(&self) -> &Device<B> {
        &self.device
    }

    /// Get raw physical device.
    pub fn physical(&self) -> &B::PhysicalDevice {
        &self.adapter.physical_device
    }

    /// Create new semaphore.
    pub fn create_semaphore(&self) -> Result<B::Semaphore, OutOfMemory> {
        profile_scope!("create_semaphore");

        self.device.create_semaphore()
    }

    /// Destroy semaphore.
    ///
    /// # Safety
    ///
    /// Semaphore must be created by this `Factory`.
    pub unsafe fn destroy_semaphore(&self, semaphore: B::Semaphore) {
        self.device.destroy_semaphore(semaphore);
    }

    /// Create new fence
    pub fn create_fence(&self, signaled: bool) -> Result<Fence<B>, OutOfMemory> {
        Fence::new(&self.device, signaled)
    }

    /// Wait for the fence become signeled.
    pub fn reset_fence(&self, fence: &mut Fence<B>) -> Result<(), OutOfMemory> {
        fence.reset(&self.device)
    }

    /// Wait for the fence become signeled.
    ///
    /// # Safety
    ///
    /// Fences must be created by this `Factory`.
    pub fn reset_fences<'a>(
        &self,
        fences: impl IntoIterator<Item = &'a mut (impl BorrowMut<Fence<B>> + 'a)>,
    ) -> Result<(), OutOfMemory> {
        let fences = fences
            .into_iter()
            .map(|f| {
                let f = f.borrow_mut();
                f.assert_device_owner(&self.device);
                assert!(f.is_signaled());
                f
            })
            .collect::<SmallVec<[_; 32]>>();
        unsafe {
            self.device.reset_fences(fences.iter().map(|f| f.raw()))?;
            fences.into_iter().for_each(|f| f.mark_reset());
        }
        Ok(())
    }

    /// Wait for the fence become signeled.
    pub fn wait_for_fence(
        &self,
        fence: &mut Fence<B>,
        timeout_ns: u64,
    ) -> Result<bool, OomOrDeviceLost> {
        profile_scope!("wait_for_fence");

        fence.assert_device_owner(&self.device);

        if let Some(fence_epoch) = fence.wait_signaled(&self.device, timeout_ns)? {
            // Now we can update epochs counter.
            let family_index = self.families_indices[fence_epoch.queue.family.index];
            let mut lock = self.epochs[family_index].write();
            let epoch = &mut lock[fence_epoch.queue.index];
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
        profile_scope!("wait_for_fences");

        let fences = fences
            .into_iter()
            .map(|f| f.borrow_mut())
            .inspect(|f| f.assert_device_owner(&self.device))
            .collect::<SmallVec<[_; 32]>>();

        if fences.is_empty() {
            return Ok(true);
        }

        let timeout = !unsafe {
            self.device
                .wait_for_fences(fences.iter().map(|f| f.raw()), wait_for, timeout_ns)
        }?;

        if timeout {
            return Ok(false);
        }

        let mut epoch_locks = SmallVec::<[_; 32]>::new();
        for fence in &fences {
            let family_id = fence.epoch().queue.family;
            while family_id.index >= epoch_locks.len() {
                epoch_locks.push(None);
            }
        }

        match wait_for {
            WaitFor::Any => {
                for fence in fences {
                    if unsafe { self.device.get_fence_status(fence.raw()) }? {
                        let epoch = unsafe { fence.mark_signaled() };
                        let family_id = epoch.queue.family;
                        let family_index = *self
                            .families_indices
                            .get(family_id.index)
                            .expect("Valid family id expected");
                        let lock = epoch_locks[family_id.index]
                            .get_or_insert_with(|| self.epochs[family_index].write());
                        let queue_epoch = &mut lock[epoch.queue.index];
                        *queue_epoch = max(*queue_epoch, epoch.epoch);
                    }
                }
            }
            WaitFor::All => {
                for fence in fences {
                    // all fences signaled
                    let epoch = unsafe { fence.mark_signaled() };
                    let family_id = epoch.queue.family;
                    let family_index = *self
                        .families_indices
                        .get(family_id.index)
                        .expect("Valid family id expected");
                    let lock = epoch_locks[family_id.index]
                        .get_or_insert_with(|| self.epochs[family_index].write());
                    let queue_epoch = &mut lock[epoch.queue.index];
                    *queue_epoch = max(*queue_epoch, epoch.epoch);
                }
            }
        }
        Ok(true)
    }

    /// Destroy fence.
    ///
    /// # Safety
    ///
    /// Fence must be created by this `Factory`.
    pub fn destroy_fence(&self, fence: Fence<B>) {
        unsafe { self.device.destroy_fence(fence.into_inner()) }
    }

    /// Create new command pool for specified family.
    pub fn create_command_pool<R>(
        &self,
        family: &Family<B>,
    ) -> Result<CommandPool<B, QueueType, R>, OutOfMemory>
    where
        R: Reset,
    {
        profile_scope!("create_command_pool");

        family.create_pool(&self.device)
    }

    /// Create new command pool for specified family.
    ///
    /// # Safety
    ///
    /// All command buffers allocated from the pool must be freed.
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
        profile_scope!("cleanup");

        let next = self.next_epochs(families);
        let complete = self.complete_epochs();
        unsafe {
            self.uploader.cleanup(&self.device);
            self.blitter.cleanup(&self.device);
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

    /// Flush blits
    pub fn flush_blits(&mut self, families: &mut Families<B>) {
        unsafe { self.blitter.flush(families) }
    }

    /// Flush uploads and cleanup unused resources.
    pub fn maintain(&mut self, families: &mut Families<B>) {
        self.flush_uploads(families);
        self.flush_blits(families);
        self.cleanup(families);
    }

    /// Create descriptor set layout with specified bindings.
    pub fn create_relevant_descriptor_set_layout(
        &self,
        bindings: Vec<DescriptorSetLayoutBinding>,
    ) -> Result<DescriptorSetLayout<B>, OutOfMemory> {
        unsafe { DescriptorSetLayout::create(&self.device, DescriptorSetInfo { bindings }) }
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
        profile_scope!("create_descriptor_sets");

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

    /// Query memory utilization.
    pub fn memory_utilization(&self) -> TotalMemoryUtilization {
        self.heaps.lock().utilization()
    }
}

with_winit! {
    impl<B> Factory<B>
    where
        B: Backend,
    {
        /// Create rendering surface from window.
        pub fn create_surface_from_winit(&mut self, window: &rendy_core::winit::window::Window) -> Surface<B> {
            profile_scope!("create_surface");
            Surface::from_winit(&self.instance, window)
        }
    }
}

impl<B> std::ops::Deref for Factory<B>
where
    B: Backend,
{
    type Target = Device<B>;

    fn deref(&self) -> &Device<B> {
        &self.device
    }
}


/// Initialize `Factory` and Queue `Families` associated with Device.
#[allow(unused)]
pub fn init<B>(
    config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
) -> Result<(Factory<B>, Families<B>), failure::Error>
where
    B: Backend,
{
    use crate::core::{identical_cast, rendy_backend};

    log::debug!("Creating factory");
    rendy_backend!(type B {
        Dx12 => {
            profile_scope!(concat!("init_factory"));
            let instance = rendy_core::dx12::Instance::create("Rendy", 1);
            Ok(identical_cast(init_with_instance(instance, config)?))
        }
        Empty => {
            profile_scope!(concat!("init_factory"));
            let instance = rendy_core::empty::Instance::create("Rendy", 1);
            Ok(identical_cast(init_with_instance(instance, config)?))
        }
        Gl => {
            failure::bail!("This function does not support GL backend. Use `init_with_instance` instead.")
        }
        Metal => {
            profile_scope!(concat!("init_factory"));
            let instance = rendy_core::metal::Instance::create("Rendy", 1);
            Ok(identical_cast(init_with_instance(instance, config)?))
        }
        Vulkan => {
            profile_scope!(concat!("init_factory"));
            let instance = rendy_core::vulkan::Instance::create("Rendy", 1);
            Ok(identical_cast(init_with_instance(instance, config)?))
        }
    })
}

/// Initialize `Factory` and Queue `Families` associated with Device
/// using existing `Instance`.
pub fn init_with_instance<B>(
    instance: impl hal::Instance<Backend = B>,
    config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
) -> Result<(Factory<B>, Families<B>), failure::Error>
where
    B: Backend,
{
    rendy_with_slow_safety_checks!(
        log::warn!("Slow safety checks are enabled! Disable them in production by enabling the 'no-slow-safety-checks' feature!")
    );
    let mut adapters = instance.enumerate_adapters();

    if adapters.is_empty() {
        failure::bail!("No physical devices found");
    }

    log::debug!(
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

    log::debug!(
        "Physical device picked: {:#?}",
        PhysicalDeviceInfo {
            name: &adapter.info.name,
            features: adapter.physical_device.features(),
            limits: adapter.physical_device.limits(),
        }
    );

    let instance = Instance::new(instance);
    let device_id = DeviceId::new(instance.id());

    let (device, families) = {
        let families = config
            .queues
            .configure(device_id, &adapter.queue_families)
            .into_iter()
            .collect::<SmallVec<[_; 16]>>();
        let (create_queues, get_queues): (SmallVec<[_; 32]>, SmallVec<[_; 32]>) = families
            .iter()
            .map(|(index, priorities)| {
                (
                    (&adapter.queue_families[index.index], priorities.as_ref()),
                    (*index, priorities.as_ref().len()),
                )
            })
            .unzip();

        log::debug!("Queues: {:#?}", get_queues);

        let Gpu { device, queue_groups } = unsafe {
            adapter
                .physical_device
                .open(&create_queues, adapter.physical_device.features())
        }?;

        let families = unsafe {
            families_from_device(device_id, queue_groups, &adapter.queue_families)
        };
        (device, families)
    };

    let device = Device::from_raw(device, device_id);

    let (types, heaps) = config
        .heaps
        .configure(&adapter.physical_device.memory_properties());
    let heaps = heaps.into_iter().collect::<SmallVec<[_; 16]>>();
    let types = types.into_iter().collect::<SmallVec<[_; 32]>>();

    log::debug!("Heaps: {:#?}\nTypes: {:#?}", heaps, types);

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
        blitter: unsafe { Blitter::new(&device, &families) }?,
        families_indices: families.indices().into(),
        epochs,
        device,
        adapter,
        instance,
    };

    Ok((factory, families))
}
