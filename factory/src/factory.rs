use {
    crate::{
        command::{
            families_from_device, CommandPool, Families, Family, FamilyId, Fence, QueueType, Reset,
        },
        config::{Config, DevicesConfigure, HeapsConfigure, QueuesConfigure},
        descriptor::DescriptorAllocator,
        memory::{Heaps, Write},
        resource::{
            buffer::{self, Buffer},
            image::{self, Image, ImageView},
            sampler::Sampler,
            set::{DescriptorSet, DescriptorSetLayout},
            Epochs, Resources,
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

/// Higher level device interface.
/// Manges memory, resources and queue families.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Factory<B: Backend> {
    descriptor_allocator: ManuallyDrop<parking_lot::Mutex<DescriptorAllocator<B>>>,
    heaps: ManuallyDrop<parking_lot::Mutex<Heaps<B>>>,
    resources: ManuallyDrop<parking_lot::RwLock<Resources<B>>>,
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
            std::ptr::read(&mut *self.resources).into_inner().dispose(
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

    /// Creates a buffer that is managed with the specified properties.
    pub fn create_buffer(
        &self,
        align: u64,
        size: u64,
        usage: impl buffer::Usage,
    ) -> Result<Buffer<B>, failure::Error> {
        let mut heaps = self.heaps.lock();
        self.resources
            .read()
            .create_buffer(&self.device, &mut heaps, align, size, usage)
    }

    /// Creates an image that is managed with the specified properties.
    pub fn create_image(
        &self,
        align: u64,
        kind: image::Kind,
        levels: image::Level,
        format: format::Format,
        tiling: image::Tiling,
        view_caps: image::ViewCapabilities,
        usage: impl image::Usage,
    ) -> Result<Image<B>, failure::Error> {
        let mut heaps = self.heaps.lock();
        self.resources.read().create_image(
            &self.device,
            &mut heaps,
            align,
            kind,
            levels,
            format,
            tiling,
            view_caps,
            usage,
        )
    }

    /// Create an image view that is managed with the specified properties
    pub fn create_image_view(
        &self,
        image: &Image<B>,
        view_kind: image::ViewKind,
        format: format::Format,
        swizzle: format::Swizzle,
        range: image::SubresourceRange,
    ) -> Result<ImageView<B>, failure::Error> {
        self.resources.read().create_image_view(
            &self.device,
            image,
            view_kind,
            format,
            swizzle,
            range,
        )
    }

    /// Create a sampler
    pub fn create_sampler(
        &self,
        filter: image::Filter,
        wrap_mode: image::WrapMode,
    ) -> Result<Sampler<B>, failure::Error> {
        self.resources
            .write()
            .create_sampler(&self.device, filter, wrap_mode)
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
        buffer: &mut Buffer<B>,
        offset: u64,
        content: &[T],
        last: Option<BufferState>,
        next: BufferState,
    ) -> Result<(), failure::Error> {
        let content_size = content.len() as u64 * std::mem::size_of::<T>() as u64;
        let mut staging = self.create_buffer(256, content_size, buffer::UploadBuffer)?;

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
        image: &mut Image<B>,
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

        let mut staging = self.create_buffer(256, content_size, buffer::UploadBuffer)?;

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

    /// Destroy target returning underlying window back to the caller.
    pub unsafe fn destroy_target(&self, target: Target<B>) {
        target.dispose(&self.device);
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
            self.resources.get_mut().cleanup(
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
    pub fn create_descriptor_set_layout(
        &self,
        bindings: Vec<DescriptorSetLayoutBinding>,
    ) -> Result<DescriptorSetLayout<B>, OutOfMemory> {
        self.resources
            .read()
            .create_descriptor_set_layout(&self.device, bindings)
    }

    // /// Destroy descriptor set layout with specified bindings.
    // pub fn destroy_descriptor_set_layout(&self, layout: DescriptorSetLayout<B>) {
    //     unsafe {
    //         layout.dispose(&self.device);
    //     }
    // }

    /// Create descriptor sets with specified layout.
    ///
    /// # Safety
    ///
    /// `layout` must be created by this `Factory`.
    ///
    pub fn create_descriptor_set(
        &self,
        layout: &DescriptorSetLayout<B>,
    ) -> Result<DescriptorSet<B>, OutOfMemory> {
        Ok(self
            .create_descriptor_sets::<SmallVec<[_; 1]>>(layout, 1)?
            .swap_remove(0))
    }

    /// Create descriptor sets with specified layout.
    ///
    /// # Safety
    ///
    /// `layout` must be created by this `Factory`.
    ///
    pub fn create_descriptor_sets<E>(
        &self,
        layout: &DescriptorSetLayout<B>,
        count: u32,
    ) -> Result<E, OutOfMemory>
    where
        E: Default + Extend<DescriptorSet<B>>,
    {
        self.resources.read().create_descriptor_sets(
            &self.device,
            &mut self.descriptor_allocator.lock(),
            layout,
            count,
        )
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
        resources: ManuallyDrop::new(parking_lot::RwLock::new(Resources::new())),
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
