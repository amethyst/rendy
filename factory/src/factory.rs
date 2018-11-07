use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

use command::{families_from_device, Family};
use memory::{Block, Heaps, MemoryUsage, Write};
use resource::{buffer::{self, Buffer}, image::{self, Image}, Resources};
use wsi::Target;

use config::{Config, HeapsConfigure, QueuesConfigure};

pub struct Factory<B: gfx_hal::Backend> {
    instance: Box<dyn gfx_hal::Instance<Backend = B>>,
    adapter: gfx_hal::Adapter<B>,
    device: B::Device,
    families: Vec<Family<B>>,
    heaps: Heaps<B>,
    resources: Resources<B>,
    relevant: relevant::Relevant,
}

impl<B> Factory<B>
where
    B: gfx_hal::Backend,
{
    /// Creates a new `Factory` based off of a `Config<Q, W>` with some `QueuesConfigure`
    /// from the specified `vk::PhysicalDevice`.
    pub fn new(instance: impl gfx_hal::Instance<Backend = B>, config: Config<impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
        let mut adapters = instance.enumerate_adapters();

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

        let factory = Factory {
            instance: Box::new(instance),
            adapter,
            device,
            families,
            heaps,
            resources: Resources::new(),
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
            family.dispose(&self.device);
        }

        unsafe {
            // All queues complete.
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
    pub unsafe fn upload_buffer(
        &mut self,
        buffer: &mut Buffer<B>,
        offset: u64,
        content: &[u8],
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
    pub unsafe fn upload_visible_buffer(
        &mut self,
        buffer: &mut Buffer<B>,
        offset: u64,
        content: &[u8],
    ) -> Result<(), failure::Error> {
        let block = buffer.block_mut();
        assert!(
            block
                .properties()
                .contains(gfx_hal::memory::Properties::CPU_VISIBLE)
        );
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

    /// Create render target from window.
    pub fn create_target(&self, window: winit::Window, image_count: u32, usage: gfx_hal::image::Usage) -> Result<Target<B>, failure::Error> {
        Target::new(
            &*self.instance,
            &self.adapter.physical_device,
            &self.device,
            window,
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

    /// Create new fence
    pub fn create_fence(&self, signaled: bool) -> Result<B::Fence, gfx_hal::device::OutOfMemory> {
        unsafe {
            gfx_hal::Device::create_fence(&self.device, signaled)
        }
    }

    /// Wait for the fence become signeled.
    /// TODO:
    /// * Add timeout.
    /// * Add multifence version.
    pub fn reset_fence(&self, fence: &B::Fence) -> Result<(), gfx_hal::device::OutOfMemory> {
        unsafe {
            // TODO: Handle device lost.
            gfx_hal::Device::reset_fences(&self.device, Some(fence))
        }
    }

    /// Wait for the fence become signeled.
    /// TODO:
    /// * Add timeout.
    /// * Add multifence version.
    pub fn wait_for_fence(&self, fence: &B::Fence) -> Result<(), gfx_hal::device::OomOrDeviceLost> {
        unsafe {
            gfx_hal::Device::wait_for_fence(&self.device, fence, !0)?;
        }
        Ok(())
    }

    // /// Inefficiently upload image data.
    // pub fn _inefficiently_upload_image(
    //     &mut self,
    //     image: &mut Image,
    //     data: &[u8],
    //     layout: vk::ImageLayout,
    // ) {
    //     let mut staging_buffer = self.create_buffer(
    //         vk::BufferCreateInfo::builder()
    //             .size(data.len() as u64)
    //             .usage(vk::BufferUsageFlags::TRANSFER_SRC)
    //             .build(),
    //         1,
    //         Upload,
    //     ).unwrap();

    //     self.upload_visible_buffer(&mut staging_buffer, 0, data).unwrap();

    //     let extent = image.extent();

    //     let command_pool = self.families[0].create_owning_pool(&self.device, crate::command::PrimaryLevel).unwrap();
    //     let command_buffer = command_pool.acquire_buffer(&self.device);
    //     let command_buffer = command_buffer.begin(&self.device, crate::command::OneShot);
    //     self.device.cmd_copy_buffer_to_image(
    //         command_buffer.raw(),
    //         staging_buffer.raw(),
    //         image.raw(),
    //         layout,
    //         &[
    //             vk::BufferImageCopy::builder()
    //                 .buffer_row_length(extent.width * 4)
    //                 .buffer_image_height(extent.height * extent.width * 4)
    //                 .image_extent(extent)
    //                 .build(),
    //         ]
    //     )
    // }
}


macro_rules! init_factory_for_backend {
    ($target:ident, $config:ident | $backend:ident @ $module:ident ? $feature:meta) => {
        [$feature]
        {
            if std::any::TypeId::of::<$backend::Backend>() == $target {
                let instance = $backend::Instance::create("Rendy", 1);
                let factory: Box<Any> = Box::new(Factory::new(instance, config));
                factory.downcast().expect(concat!("`", stringify!($backend), "::Backend::Surface` must be `", stringify!($backend), "::Surface`"));
            }
        }
    };

    ($target:ident, $config:ident $(| $backend:ident @ $module:ident ? $feature:meta)*) => {
        $(init_factory_for_backend!($target, $config | $backend @ $module ? $feature));*
    };

    ($target:ident, $config:ident) => {
        init_factory_for_backend!($target, $config
            | gfx_backend_dx12 @ dx12 ? cfg(feature = "gfx-backend-dx12")
            | gfx_backend_metal @ metal ? cfg(feature = "gfx-backend-metal")
            | gfx_backend_vulkan @ vulkan ? cfg(feature = "gfx-backend-vulkan")
        );
    };
}

/// Init factory.
pub fn init<B>(config: Config<impl HeapsConfigure, impl QueuesConfigure>) -> Result<Factory<B>, failure::Error>
where
    B: gfx_hal::Backend,
{
    let type_id = std::any::TypeId::of::<B>();
    init_factory_for_backend!(type_id, config);
    panic!("Undefined backend requested. Make sure feature for required backends are enabled.")
}
