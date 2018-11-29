
use command::{families_from_device, Family, Reset, CommandPool};
use memory::{Block, Heaps, Write};
use resource::{buffer::{self, Buffer}, image::{self, Image}, Resources};
use wsi::{Surface, Target};

use config::{Config, HeapsConfigure, QueuesConfigure};

pub struct Factory<B: gfx_hal::Backend> {
    instance: Box<dyn std::any::Any>,
    adapter: gfx_hal::Adapter<B>,
    device: B::Device,
    heaps: Heaps<B>,
    resources: Resources<B>,
    families: Vec<Family<B>>,
    families_indices: std::collections::HashMap<gfx_hal::queue::QueueFamilyId, usize>,
    relevant: relevant::Relevant,
}

impl<B> Factory<B>
where
    B: gfx_hal::Backend,
{
    /// Creates a new `Factory` based off of a `Config<Q, W>` with some `QueuesConfigure`
    /// from the specified `vk::PhysicalDevice`.
    pub fn init(instance: impl gfx_hal::Instance<Backend = B>, config: Config<impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
        let adapters = instance.enumerate_adapters();

        if adapters.is_empty() {
            failure::bail!("No physical devices found");
        }

        log::info!("Physical devices:\n{:#?}", adapters.iter().map(|adapter| &adapter.info).collect::<smallvec::SmallVec<[_; 32]>>());

        let adapter = adapters
            .into_iter()
            .min_by_key(|adapter| match adapter.info.device_type {
                gfx_hal::adapter::DeviceType::DiscreteGpu => 0,
                gfx_hal::adapter::DeviceType::IntegratedGpu => 1,
                gfx_hal::adapter::DeviceType::VirtualGpu => 2,
                gfx_hal::adapter::DeviceType::Cpu => 3,
                _ => 4,
            }).unwrap();

        log::info!("Physical device picked: {}", adapter.info.name);

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
            heaps,
            resources: Resources::new(),
            families,
            families_indices,
            relevant: relevant::Relevant,
        };

        Ok(factory)
    }

    pub fn wait_idle(&self) -> Result<(), gfx_hal::error::HostExecutionError> {
        gfx_hal::Device::wait_idle(&self.device)
    }

    pub fn dispose(mut self) {
        let _ = self.wait_idle();
        for family in self.families {
            family.dispose();
        }

        unsafe {
            // All queues complete.
            self.resources.cleanup(&self.device, &mut self.heaps);
            self.resources.cleanup(&self.device, &mut self.heaps);
        }

        self.heaps.dispose(&self.device);

        drop(self.device);
        drop(self.instance);

        self.relevant.dispose();
        // trace!("Factory destroyed");
    }

    /// Creates a buffer that is managed with the specified properties.
    pub fn create_buffer(
        &mut self,
        align: u64,
        size: u64,
        usage: impl buffer::Usage,
    ) -> Result<Buffer<B>, failure::Error> {
        self.resources
            .create_buffer(
                &self.device,
                &mut self.heaps,
                align,
                size,
                usage
            )
    }

    /// Upload buffer content.
    ///
    /// # Safety
    ///
    /// * Buffer must be created by this `Factory`.
    /// * Caller must ensure that device won't write to or read from
    /// the memory region occupied by this buffer.
    pub unsafe fn upload_buffer<T>(
        &mut self,
        buffer: &mut Buffer<B>,
        offset: u64,
        content: &[T],
        family: gfx_hal::queue::QueueFamilyId,
        access: gfx_hal::buffer::Access,
    ) -> Result<(), failure::Error> {
        if buffer
            .block()
            .properties()
            .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
        {
            self.upload_visible_buffer(buffer, offset, content)
        } else {
            unimplemented!("Staging is not supported yet");
        }
    }

    /// Update buffer bound to host visible memory.vk::AccessFlags.
    ///
    /// # Safety
    ///
    /// * Caller must ensure that device won't write to or read from
    /// the memory region occupied by this buffer.
    pub unsafe fn upload_visible_buffer<T>(
        &mut self,
        buffer: &mut Buffer<B>,
        offset: u64,
        content: &[T],
    ) -> Result<(), failure::Error> {
        let block = buffer.block_mut();
        assert!(
            block
                .properties()
                .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
        );

        let content = unsafe {
            std::slice::from_raw_parts(content.as_ptr() as *const u8, content.len() * std::mem::size_of::<T>())
        };

        let mut mapped = block.map(&self.device, offset..offset + content.len() as u64)?;
        mapped
            .write(&self.device, 0..content.len() as u64)?
            .write(content);
        Ok(())
    }

    /// Creates an image that is mananged with the specified properties.
    pub fn create_image(
        &mut self,
        align: u64,
        kind: gfx_hal::image::Kind,
        levels: gfx_hal::image::Level,
        format: gfx_hal::format::Format,
        tiling: gfx_hal::image::Tiling,
        view_caps: gfx_hal::image::ViewCapabilities,
        usage: impl image::Usage,
    ) -> Result<Image<B>, failure::Error> {
        self.resources
            .create_image(
                &self.device,
                &mut self.heaps,
                align,
                kind,
                levels,
                format,
                tiling,
                view_caps,
                usage,
            )
    }

    /// Create rendering surface from window.
    pub fn create_surface(&self, window: winit::Window) -> Surface<B> {
        Surface::new(
            &self.instance,
            window,
        )
    }

    /// Create target out of rendering surface.
    pub fn create_target(&self, surface: Surface<B>, image_count: u32, usage: gfx_hal::image::Usage) -> Result<Target<B>, failure::Error> {
        surface.into_target(
            &self.adapter.physical_device,
            &self.device,
            image_count,
            usage,
        )
    }

    /// Destroy target returning underlying window back to the caller.
    pub unsafe fn destroy_target(&self, target: Target<B>) -> winit::Window {
        let window = target.dispose(&self.device);
        window
    }

    /// Get queue families of the factory.
    pub fn families(&self) -> &[Family<B>] {
        &self.families
    }

    /// Get queue families of the factory.
    pub fn families_mut(&mut self) -> &mut [Family<B>] {
        &mut self.families
    }

    /// Get queue families of the factory.
    pub fn family(&self, index: gfx_hal::queue::QueueFamilyId) -> &Family<B> {
        &self.families[self.families_indices[&index]]
    }

    /// Get queue families of the factory.
    pub fn family_mut(&mut self, index: gfx_hal::queue::QueueFamilyId) -> &mut Family<B> {
        &mut self.families[self.families_indices[&index]]
    }

    /// Get surface support for family.
    pub fn target_support(&self, family: gfx_hal::queue::QueueFamilyId, target: &Target<B>) -> bool {
        unsafe {
            gfx_hal::Surface::supports_queue_family(target.surface(), &self.adapter.queue_families[family.0])
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
    pub fn create_command_pool<R>(&self, family: gfx_hal::queue::QueueFamilyId, reset: R) -> Result<CommandPool<B, gfx_hal::QueueType, R>, failure::Error>
    where
        R: Reset,
    {
        self.family(family)
            .create_pool(&self.device, reset)
            .map_err(Into::into)
    }
    
    /// Create new command pool for specified family.
    pub unsafe fn destroy_command_pool<R>(&self, pool: CommandPool<B, gfx_hal::QueueType, R>)
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
                        let factory: Box<std::any::Any> = Box::new(Factory::init(instance, $config)?);
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
    pub fn new(config: Config<impl HeapsConfigure, impl QueuesConfigure>) -> Result<Factory<B>, failure::Error> {
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