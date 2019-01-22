
use crate::{
    command::{families_from_device, Family, Reset, CommandPool, FamilyId},
    memory::{Heaps, Write},
    resource::{buffer::{self, Buffer}, image::{self, Image}, Resources},
    wsi::{Surface, Target},
    config::{Config, HeapsConfigure, QueuesConfigure, DevicesConfigure},
    upload::{Uploader, BufferState, ImageState, ImageStateOrLayout},
};

/// Higher level device interface.
/// Manges memory, resources and queue families.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Factory<B: gfx_hal::Backend> {
    #[derivative(Debug = "ignore")] instance: Box<dyn std::any::Any>,
    #[derivative(Debug = "ignore")] adapter: gfx_hal::Adapter<B>,
    #[derivative(Debug = "ignore")] device: B::Device,
    heaps: parking_lot::Mutex<Heaps<B>>,
    resources: parking_lot::RwLock<Resources<B>>,
    families: Vec<Family<B>>,
    families_indices: std::collections::HashMap<FamilyId, usize>,
    uploads: Uploader<B>,
    relevant: relevant::Relevant,
}

impl<B> Factory<B>
where
    B: gfx_hal::Backend,
{
    /// Creates a new `Factory` based off of a `Config<Q, W>` with some `QueuesConfigure`
    /// from the specified `vk::PhysicalDevice`.
    pub fn init(instance: impl gfx_hal::Instance<Backend = B>, config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
        let mut adapters = instance.enumerate_adapters();

        if adapters.is_empty() {
            failure::bail!("No physical devices found");
        }

        log::info!("Physical devices:\n{:#?}", adapters.iter().map(|adapter| &adapter.info).collect::<smallvec::SmallVec<[_; 32]>>());

        let picked = config.devices.pick(&adapters);
        if picked >= adapters.len() {
            panic!("Physical device pick config returned index out of bound");
        }
        let adapter = adapters.swap_remove(picked);

        #[derive(Debug)]
        struct PhysicalDeviceInfo<'a> {
            name: &'a str,
            features: gfx_hal::Features,
            limits: gfx_hal::Limits,
        }

        log::info!("Physical device picked: {:#?}", PhysicalDeviceInfo {
            name: &adapter.info.name,
            features: gfx_hal::adapter::PhysicalDevice::features(&adapter.physical_device),
            limits: gfx_hal::adapter::PhysicalDevice::limits(&adapter.physical_device),
        });

        let (device, families) = {
            let families = config.queues.configure(&adapter.queue_families)
                .into_iter()
                .collect::<smallvec::SmallVec<[_; 16]>>();
            let (create_queues, get_queues): (smallvec::SmallVec<[_; 32]>, smallvec::SmallVec<[_; 32]>) = families
                .iter()
                .map(|(index, priorities)| {
                    ((&adapter.queue_families[index.0], priorities.as_ref()), (*index, priorities.as_ref().len()))
                }).unzip();

            log::info!("Queues: {:#?}", get_queues);

            let gfx_hal::Gpu { device, mut queues } = unsafe {
                gfx_hal::PhysicalDevice::open(&adapter.physical_device, &create_queues)
            }?;

            let families = unsafe { families_from_device(&mut queues, get_queues, &adapter.queue_families) };
            (device, families)
        };

        let (types, heaps) = config.heaps.configure(&gfx_hal::PhysicalDevice::memory_properties(&adapter.physical_device));
        let heaps = heaps.into_iter().collect::<smallvec::SmallVec<[_; 16]>>();
        let types = types.into_iter().collect::<smallvec::SmallVec<[_; 32]>>();

        log::info!("Heaps: {:#?}\nTypes: {:#?}", heaps, types);

        let heaps = unsafe { Heaps::new(types, heaps) };

        let families_indices = families.iter().enumerate().map(|(i, f)| (f.index(), i)).collect();

        let factory = Factory {
            instance: Box::new(instance),
            adapter,
            device,
            heaps: parking_lot::Mutex::new(heaps),
            resources: parking_lot::RwLock::new(Resources::new()),
            uploads: Uploader::new(families.len()),
            families,
            families_indices,
            relevant: relevant::Relevant,
        };

        Ok(factory)
    }

    /// Wait for whole device become idle.
    /// This function is very heavy and
    /// usually used only for teardown.
    pub fn wait_idle(&self) -> Result<(), gfx_hal::error::HostExecutionError> {
        gfx_hal::Device::wait_idle(&self.device)
    }

    /// Dispose of the `Factory`.
    pub fn dispose(mut self) {
        let _ = self.wait_idle();
        for family in self.families {
            family.dispose();
        }

        unsafe {
            // All queues complete.
            self.resources.get_mut().cleanup(&self.device, self.heaps.get_mut());
            self.resources.get_mut().cleanup(&self.device, self.heaps.get_mut());
        }

        self.heaps.into_inner().dispose(&self.device);

        drop(self.device);
        drop(self.instance);

        self.relevant.dispose();
        log::trace!("Factory destroyed");
    }

    /// Creates a buffer that is managed with the specified properties.
    pub fn create_buffer(
        &self,
        align: u64,
        size: u64,
        usage: impl buffer::Usage,
    ) -> Result<Buffer<B>, failure::Error> {
        self.resources.read()
            .create_buffer(
                &self.device,
                &mut self.heaps.lock(),
                align,
                size,
                usage
            )
    }

    /// Creates an image that is mananged with the specified properties.
    pub fn create_image(
        &self,
        align: u64,
        kind: gfx_hal::image::Kind,
        levels: gfx_hal::image::Level,
        format: gfx_hal::format::Format,
        tiling: gfx_hal::image::Tiling,
        view_caps: gfx_hal::image::ViewCapabilities,
        usage: impl image::Usage,
    ) -> Result<Image<B>, failure::Error> {
        self.resources.read()
            .create_image(
                &self.device,
                &mut self.heaps.lock(),
                align,
                kind,
                levels,
                format,
                tiling,
                view_caps,
                usage,
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
        let content = unsafe {
            std::slice::from_raw_parts(content.as_ptr() as *const u8, content.len() * std::mem::size_of::<T>())
        };

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
        let mut staging = self.create_buffer(
            256,
            content_size,
            buffer::UploadBuffer,
        )?;

        self.upload_visible_buffer(&mut staging, 0, content)?;

        let family_index = self.families_indices[&next.queue.family()];
        self.uploads.families[family_index]
            .lock()
            .upload_buffer(
                &self.device,
                &self.families[family_index],
                buffer,
                offset,
                staging,
                last,
                next,
            )
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
        image_layers: gfx_hal::image::SubresourceLayers,
        image_offset: gfx_hal::image::Offset,
        image_extent: gfx_hal::image::Extent,
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
        let texels_count = (image_extent.width / format_desc.dim.0 as u32) as u64 * (image_extent.height / format_desc.dim.1 as u32) as u64 * image_extent.depth as u64;
        let total_bytes = (format_desc.bits as u64 / 8) * texels_count;
        assert_eq!(
            total_bytes, content_size,
            "Size of must match size of the image region"
        );

        let mut staging = self.create_buffer(
            256,
            content_size,
            buffer::UploadBuffer,
        )?;

        self.upload_visible_buffer(&mut staging, 0, content)?;

        let family_index = self.families_indices[&next.queue.family()];
        self.uploads.families[family_index].lock()
            .upload_image(
                &self.device,
                &self.families[family_index],
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
    pub fn create_surface(&self, window: std::sync::Arc<winit::Window>) -> Surface<B> {
        Surface::new(
            &self.instance,
            window,
        )
    }

    /// Destroy surface returning underlying window back to the caller.
    pub unsafe fn destroy_surface(&self, surface: Surface<B>) {
        drop(surface);
    }

    /// Create target out of rendering surface.
    pub fn create_target(&self, surface: Surface<B>, image_count: u32, usage: gfx_hal::image::Usage) -> Result<Target<B>, failure::Error> {
        unsafe {
            surface.into_target(
                &self.adapter.physical_device,
                &self.device,
                image_count,
                usage,
            )
        }
    }

    /// Destroy target returning underlying window back to the caller.
    pub unsafe fn destroy_target(&self, target: Target<B>) {
        target.dispose(&self.device);
    }

    /// Get queue families of the factory.
    pub fn families(&self) -> &[Family<B>] {
        &self.families
    }

    /// Get queue families of the factory.
    pub fn family(&self, id: FamilyId) -> &Family<B> {
        &self.families[self.families_indices[&id]]
    }

    /// Get queue families of the factory.
    /// This function also flushes all pending uploads for the queue.
    pub unsafe fn family_mut(&mut self, id: FamilyId) -> &mut Family<B> {
        let family_index = self.families_indices[&id];
        let family = &mut self.families[family_index];

        let family_uploads = self.uploads.families[family_index].get_mut();
        
        family_uploads.flush(family);

        family
    }

    /// Get surface support for family.
    pub fn surface_support(&self, family: FamilyId, surface: &B::Surface) -> bool {
        unsafe {
            gfx_hal::Surface::supports_queue_family(surface, &self.adapter.queue_families[family.0])
        }
    }

    /// Get device.
    pub fn device(&self) -> &impl gfx_hal::Device<B> {
        &self.device
    }

    /// Get physical device.
    pub fn physical(&self) -> &impl gfx_hal::PhysicalDevice<B> {
        &self.adapter.physical_device
    }

    /// Create new semaphore
    pub fn create_semaphore(&self) -> Result<B::Semaphore, gfx_hal::device::OutOfMemory> {
        unsafe {
            gfx_hal::Device::create_semaphore(&self.device)
        }
    }

    /// Destroy semaphore
    pub fn destroy_semaphore(&self, semaphore: B::Semaphore) {
        unsafe {
            gfx_hal::Device::destroy_semaphore(&self.device, semaphore);
        }
    }

    /// Create new fence
    pub fn create_fence(&self, signaled: bool) -> Result<B::Fence, gfx_hal::device::OutOfMemory> {
        unsafe {
            gfx_hal::Device::create_fence(&self.device, signaled)
        }
    }

    /// Wait for the fence become signeled.
    pub fn reset_fence(&self, fence: &B::Fence) -> Result<(), gfx_hal::device::OutOfMemory> {
        unsafe {
            gfx_hal::Device::reset_fence(&self.device, fence)
        }
    }

    /// Wait for the fence become signeled.
    pub fn reset_fences(&self, fences: impl IntoIterator<Item = impl std::borrow::Borrow<B::Fence>>) -> Result<(), gfx_hal::device::OutOfMemory> {
        unsafe {
            gfx_hal::Device::reset_fences(&self.device, fences)
        }
    }

    /// Wait for the fence become signeled.
    pub fn wait_for_fence(&self, fence: &B::Fence, timeout_ns: u64) -> Result<bool, gfx_hal::device::OomOrDeviceLost> {
        unsafe {
            gfx_hal::Device::wait_for_fence(&self.device, fence, timeout_ns)
        }
    }

    /// Wait for the fences become signeled.
    pub fn wait_for_fences(&self, fences: impl IntoIterator<Item = impl std::borrow::Borrow<B::Fence>>, wait_for: gfx_hal::device::WaitFor, timeout_ns: u64) -> Result<bool, gfx_hal::device::OomOrDeviceLost> {
        unsafe {
            gfx_hal::Device::wait_for_fences(&self.device, fences, wait_for, timeout_ns)
        }
    }

    /// Destroy fence.
    pub unsafe fn destroy_fence(&self, fence: B::Fence) {
        gfx_hal::Device::destroy_fence(&self.device, fence)
    }
    
    /// Create new command pool for specified family.
    pub fn create_command_pool<R>(&self, family: FamilyId) -> Result<CommandPool<B, gfx_hal::QueueType, R>, failure::Error>
    where
        R: Reset,
    {
        self.family(family)
            .create_pool(&self.device)
            .map_err(Into::into)
    }
    
    /// Create new command pool for specified family.
    pub unsafe fn destroy_command_pool<C, R>(&self, pool: CommandPool<B, C, R>)
    where
        R: Reset,
    {
        pool.dispose(&self.device);
    }
}

macro_rules! init_factory_for_backend {
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
                        let factory: Box<dyn std::any::Any> = Box::new(Factory::init(instance, $config)?);
                        return Ok(*factory.downcast::<Factory<$target>>().unwrap());
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
        init_factory_for_backend!(match $target, $config
            | gfx_backend_empty @ cfg(feature = "empty")
            | gfx_backend_dx12 @ cfg(feature = "dx12")
            | gfx_backend_metal @ cfg(feature = "metal")
            | gfx_backend_vulkan @ cfg(feature = "vulkan")
        );
    }};
}

impl<B> Factory<B>
where
    B: gfx_hal::Backend,
{
    /// Init factory.
    #[allow(unused_variables)]
    pub fn new(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Factory<B>, failure::Error> {
        log::debug!("Creating factory");
        init_factory_for_backend!(B, config)
    }
}

#[doc(hidden)]
impl<B> std::ops::Deref for Factory<B>
where
    B: gfx_hal::Backend,
{
    type Target = B::Device;

    fn deref(&self) -> &B::Device {
        &self.device
    }
}