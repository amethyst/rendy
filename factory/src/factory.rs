use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

use ash::{
    extensions::{Surface, Swapchain},
    version::{DeviceV1_0, EntryV1_0, InstanceV1_0, V1_0},
    vk,
    Device, Entry, Instance, LoadingError,
};
use failure::Error;
use relevant::Relevant;
use smallvec::SmallVec;
use winit::Window;

use command::{Family, FamilyIndex, families_from_device};
use memory::{Block, Heaps, MemoryError, MemoryUsage, Write};
use resource::{buffer::Buffer, image::Image, Resources};
use wsi::{NativeSurface, Target};

use config::{Config, HeapsConfigure, QueuesConfigure};
use queue::Queue;

#[derive(Debug, Fail)]
#[fail(display = "{:#?}", _0)]
pub struct EntryError(LoadingError);

#[derive(Debug)]
struct PhysicalDeviceInfo {
    handle: vk::PhysicalDevice,
    properties: vk::PhysicalDeviceProperties,
    memory: vk::PhysicalDeviceMemoryProperties,
    queues: Vec<vk::QueueFamilyProperties>,
    features: vk::PhysicalDeviceFeatures,
    extensions: Vec<vk::ExtensionProperties>,
}

/// The `Factory<D>` type represents the overall creation type for `rendy`.
pub struct Factory {
    instance: Instance<V1_0>,
    physical: PhysicalDeviceInfo,
    device: Device<V1_0>,
    families: Vec<Family>,
    heaps: Heaps,
    resources: Resources,
    surface: Surface,
    swapchain: Swapchain,
    native_surface: NativeSurface,
    relevant: Relevant,
}

impl Factory {
    /// Creates a new `Factory` based off of a `Config<Q, W>` with some `QueuesConfigure`
    /// from the specified `vk::PhysicalDevice`.
    pub fn new(config: Config<impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, Error> {
        let entry = Entry::<V1_0>::new().map_err(EntryError)?;

        let layers = entry.enumerate_instance_layer_properties()?;
        debug!("Available layers:\n{:#?}", layers);

        let extensions = entry.enumerate_instance_extension_properties()?;
        debug!("Available extensions:\n{:#?}", extensions);

        let instance = unsafe {
            // Only present layers and extensions are enabled.
            // Other parameters trivially valid.
            entry.create_instance(
                &vk::InstanceCreateInfo::builder()
                    .application_info(
                        &vk::ApplicationInfo::builder()
                            .application_name(&CString::new(config.app_name)?)
                            .application_version(config.app_version)
                            .engine_name(CStr::from_bytes_with_nul_unchecked(b"rendy\0"))
                            .engine_version(1)
                            .api_version(vk_make_version!(1, 0, 0))
                            .build(),
                    )
                    .enabled_extension_names(&extensions_to_enable(&extensions)?)
                    .build(),
                None,
            )
        }?;
        // trace!("Instance created");

        let surface = Surface::new(&entry, &instance)
            .map_err(|missing| format_err!("{:#?} functions are missing", missing))?;
        let native_surface = NativeSurface::new(&entry, &instance)
            .map_err(|missing| format_err!("{:#?} functions are missing", missing))?;

        let mut physicals = unsafe { 
            // Instance is valid.
            // Physical device handlers are valid (enumerated from instance).
            instance
                .enumerate_physical_devices()?
                .into_iter()
                .map(|p| PhysicalDeviceInfo {
                    handle: p,
                    properties: instance.get_physical_device_properties(p),
                    memory: instance.get_physical_device_memory_properties(p),
                    queues: instance.get_physical_device_queue_family_properties(p),
                    features: instance.get_physical_device_features(p),
                    extensions: instance.enumerate_device_extension_properties(p).unwrap(),
                })
        }.collect::<Vec<_>>();

        debug!("Physical devices:\n{:#?}", physicals);

        physicals.retain(|p| match extensions_to_enable(&p.extensions) {
            Ok(_) => true,
            Err(missing) => {
                // trace!("{:#?} missing extensions {:#?}", p, missing);
                false
            }
        });

        let physical = physicals
            .into_iter()
            .min_by_key(|info| match info.properties.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                vk::PhysicalDeviceType::CPU => 3,
                _ => 4,
            }).ok_or(format_err!("No suitable physical devices found"))?;

        let device_name = unsafe {
            // Pointer is valid.
            CStr::from_ptr(&physical.properties.device_name[0])
                .to_string_lossy()
        };

        debug!("Physical device picked: {}", device_name);

        let families = config.queues.configure(&physical.queues);

        let (create_queues, get_queues): (SmallVec<[_; 32]>, SmallVec<[_; 32]>) = families
            .into_iter()
            .map(|(index, priorities)| {
                let info = vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(index.0)
                    .queue_priorities(priorities.as_ref())
                    .build();
                let get = (index, priorities.as_ref().len() as u32);
                (info, get)
            }).unzip();

        debug!("Queues: {:#?}", get_queues);

        let device = unsafe {
            instance.create_device(
                physical.handle,
                &vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&create_queues)
                    .enabled_extension_names(&extensions_to_enable(&physical.extensions).unwrap())
                    // .enabled_features(&physical.features)
                    .build(),
                None,
            )
        }?;

        let swapchain = Swapchain::new(&instance, &device)
            .map_err(|missing| format_err!("{:#?} functions are missing", missing))?;

        let (types, heaps) = config.heaps.configure(&physical.memory);
        let heaps = heaps.into_iter().collect::<SmallVec<[_; 16]>>();
        let types = types.into_iter().collect::<SmallVec<[_; 32]>>();

        debug!("Heaps: {:#?}\nTypes: {:#?}", heaps, types);

        let heaps = unsafe { Heaps::new(types, heaps) };

        let families = unsafe { families_from_device(&device, get_queues, &physical.queues) };

        let factory = Factory {
            instance,
            physical,
            device,
            families,
            heaps,
            resources: Resources::new(),
            surface,
            swapchain,
            native_surface,
            relevant: Relevant,
        };

        // trace!("Factory created");

        Ok(factory)
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
    }

    pub fn dispose(mut self) {
        self.wait_idle();
        for family in self.families {
            family.dispose(&self.device);
        }

        unsafe {
            // All queues complete.
            self.resources.cleanup(&self.device, &mut self.heaps);
        }

        self.heaps.dispose(&self.device);
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }

        self.relevant.dispose();
        // trace!("Factory destroyed");
    }

    /// Creates a buffer that is managed with the specified properties.
    pub fn create_buffer(
        &mut self,
        info: vk::BufferCreateInfo,
        align: u64,
        memory_usage: impl MemoryUsage,
    ) -> Result<Buffer, MemoryError> {
        self.resources
            .create_buffer(&self.device, &mut self.heaps, info, align, memory_usage)
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
        buffer: &mut Buffer,
        offset: u64,
        content: &[u8],
        family: FamilyIndex,
        access: vk::AccessFlags,
    ) -> Result<(), Error> {
        if buffer.block().properties().subset(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            self.upload_visible_buffer(buffer, offset, content, family, access)
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
        buffer: &mut Buffer,
        offset: u64,
        content: &[u8],
        _family: FamilyIndex,
        _access: vk::AccessFlags,
    ) -> Result<(), Error> {
        let block = buffer.block_mut();
        assert!(block.properties().subset(vk::MemoryPropertyFlags::HOST_VISIBLE));
        let mut mapped = block.map(&self.device, offset .. offset + content.len() as u64)?;
        mapped.write(&self.device, 0 .. content.len() as u64)?.write(content);

        Ok(())
    }

    /// Creates an image that is mananged with the specified properties.
    pub fn create_image(
        &mut self,
        info: vk::ImageCreateInfo,
        align: u64,
        memory_usage: impl MemoryUsage,
    ) -> Result<Image, MemoryError> {
        self.resources
            .create_image(&self.device, &mut self.heaps, info, align, memory_usage)
    }

    /// Create render target from window.
    pub fn create_target(&self, window: Window, image_count: u32) -> Result<Target, Error> {
        Target::new(
            window,
            image_count,
            self.physical.handle,
            &self.native_surface,
            &self.surface,
            &self.swapchain,
        )
    }

    pub fn destroy_target(&self, target: Target) -> Window {
        unsafe {
            let (window, surface, swapchain) = target.dispose();
            self.swapchain.destroy_swapchain_khr(swapchain, None);
            // trace!("Swapchain destroyed");
            self.surface.destroy_surface_khr(surface, None);
            // trace!("Surface destroyed");
            window
        }
    }

    pub fn families(&self) -> &[Family] {
        &self.families
    }

    pub fn queue(&mut self, family: FamilyIndex, queue: usize) -> Queue<'_> {
        let raw = self.families[family.0 as usize].queues()[queue];
        Queue {
            fp: self.device().fp_v1_0(),
            raw,
        }
    }

    /// Get surface support for family.
    pub fn target_support(&self, family: FamilyIndex, target: &Target) -> bool {
        unsafe { 
            let surface = target.surface();
            self.surface.get_physical_device_surface_support_khr(self.physical.handle, family.0, surface)
        }
    }

    /// Get device.
    pub fn device(&self) -> &impl DeviceV1_0 {
        &self.device
    }

    /// Get physical device.
    pub fn physical(&self) -> vk::PhysicalDevice {
        self.physical.handle
    }

    /// Get surface capabilities.
    pub fn surface_capabilities(&self, target: &Target) -> Result<vk::SurfaceCapabilitiesKHR, Error> {
        unsafe {
            self.surface.get_physical_device_surface_capabilities_khr(self.physical.handle, target.surface())
        }.map_err(Error::from)
    }

    /// Create new semaphore
    pub fn create_semaphore(&self) -> vk::Semaphore {
        unsafe {
            self.device.create_semaphore(
                &vk::SemaphoreCreateInfo::builder()
                    .build(),
                None
            )
        }.expect("Panic on OOM")
    }

    /// Create new fence
    pub fn create_fence(&self, signaled: bool) -> vk::Fence {
        unsafe {
            self.device.create_fence(
                &vk::FenceCreateInfo::builder()
                    .flags(if signaled {
                        vk::FenceCreateFlags::SIGNALED
                    } else {
                        vk::FenceCreateFlags::empty()
                    })
                    .build(),
                None
            )
        }.expect("Panic on OOM")
    }

    /// Wait for the fence become signeled.
    /// TODO:
    /// * Add timeout.
    /// * Add multifence version.
    pub fn wait_for_fence(&self, fence: vk::Fence) {
        unsafe {
            // TODO: Handle device lost.
            self.device.wait_for_fences(&[fence], true, !0).unwrap();
        }
    }
}


unsafe fn extension_name_cstr(e: &vk::ExtensionProperties) -> &CStr {
    CStr::from_ptr(e.extension_name[..].as_ptr())
}

fn extensions_to_enable(available: &[vk::ExtensionProperties]) -> Result<Vec<*const c_char>, Error> {
    let names = vec![
        Surface::name().as_ptr(),
        Swapchain::name().as_ptr(),
        NativeSurface::name().as_ptr(),
    ];

    let not_found = unsafe {
        names
            .iter()
            .cloned()
            .filter_map(|name| {
                let cstr_name = CStr::from_ptr(name);
                if available
                    .iter()
                    .find(|e| extension_name_cstr(e) == cstr_name)
                    .is_none()
                {
                    Some(cstr_name)
                } else {
                    None
                }
            }).collect::<Vec<_>>()
    };

    if not_found.is_empty() {
        Ok(names)
    } else {
        Err(format_err!(
            "Extensions {:#?} are not available: {:#?}",
            not_found,
            available
        ))
    }
}
