
use std::{ffi::{CStr, CString}, os::raw::c_char, cmp::{max, min}};

use ash::{
    Device,
    Instance,
    Entry,
    LoadingError,
    extensions::{
        Surface,
        Swapchain,
    },
    version::{
        DeviceV1_0,
        EntryV1_0,
        InstanceV1_0,
        V1_0,
    },
    vk::{
        AccessFlags,
        BufferUsageFlags,
        BufferCreateInfo,
        ImageCreateFlags,
        ImageCreateInfo,
        ImageType,
        Extent3D,
        Format,
        SampleCountFlags,
        ImageTiling,
        ImageUsageFlags,
        SharingMode,
        ImageLayout,
        PhysicalDevice,
        InstanceCreateInfo,
        ApplicationInfo,
        ExtensionProperties,
        PhysicalDeviceMemoryProperties,
        PhysicalDeviceProperties,
        QueueFamilyProperties,
        PhysicalDeviceType,
        DeviceCreateInfo,
        DeviceQueueCreateInfo,
        PhysicalDeviceFeatures,
        SwapchainCreateInfoKHR,
    },
};
use failure::Error;
use relevant::Relevant;
use smallvec::SmallVec;
use winit::Window;

use command::{Families, FamilyId};
use memory::{Heaps, MemoryError, MemoryUsage};
use resource::{
    buffer::Buffer,
    image::Image,
    Resources,
};

use config::{Config, HeapsConfigure, QueuesConfigure};
use renderer::{RendererDesc, RendererBuilder};
use wsi::{NativeSurface, Target};

#[derive(Debug, Fail)]
#[fail(display = "{:#?}", _0)]
pub struct EntryError(LoadingError);

#[derive(Debug)]
struct PhysicalDeviceInfo {
    handle: PhysicalDevice,
    properties: PhysicalDeviceProperties,
    memory: PhysicalDeviceMemoryProperties,
    queues: Vec<QueueFamilyProperties>,
    features: PhysicalDeviceFeatures,
    extensions: Vec<ExtensionProperties>,
}

/// The `Factory<D>` type represents the overall creation type for `rendy`.
pub struct Factory {
    instance: Instance<V1_0>,
    physical: PhysicalDeviceInfo,
    device: Device<V1_0>,
    families: Families,
    heaps: Heaps,
    resources: Resources,
    surface: Surface,
    swapchain: Swapchain,
    native_surface: NativeSurface,
    relevant: Relevant,
}

impl Factory {
    /// Creates a new `Factory` based off of a `Config<Q, W>` with some `QueuesConfigure`
    /// from the specified `PhysicalDevice`.
    pub fn new(config: Config<impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, Error> {
        let entry = Entry::<V1_0>::new().map_err(EntryError)?;

        let layers = entry.enumerate_instance_layer_properties()?;
        debug!("Available layers:\n{:#?}", layers);

        let extensions = entry.enumerate_instance_extension_properties()?;
        debug!("Available extensions:\n{:#?}", extensions);

        let instance = unsafe {
            entry.create_instance(
                &InstanceCreateInfo::builder()
                    .application_info(
                        &ApplicationInfo::builder()
                            .application_name(&CString::new(config.app_name)?)
                            .application_version(config.app_version)
                            .build()
                    )
                    .enabled_extension_names(&extensions_to_enable(&extensions)?)
                    .build(),
                None,
            )
        }?;
        trace!("Instance created");

        let surface = Surface::new(&entry, &instance)
            .map_err(|missing| {
                format_err!("{:#?} functions are missing", missing)
            })?;
        let native_surface = NativeSurface::new(&entry, &instance)
            .map_err(|missing| {
                format_err!("{:#?} functions are missing", missing)
            })?;

        let mut physicals = unsafe {
            instance.enumerate_physical_devices()?.into_iter().map(|p| PhysicalDeviceInfo {
                handle: p,
                properties: instance.get_physical_device_properties(p),
                memory: instance.get_physical_device_memory_properties(p),
                queues: instance.get_physical_device_queue_family_properties(p),
                features: instance.get_physical_device_features(p),
                extensions: instance.enumerate_device_extension_properties(p).unwrap(),
            })
        }.collect::<Vec<_>>();

        debug!("Physical devices:\n{:#?}", physicals);

        physicals.retain(|p| {
            match extensions_to_enable(&p.extensions) {
                Ok(_) => true,
                Err(missing) => {
                    trace!("{:#?} missing extensions {:#?}", p, missing);
                    false
                }
            }
        });

        let physical = physicals.into_iter().min_by_key(|info| match info.properties.device_type {
            PhysicalDeviceType::DISCRETE_GPU => 0,
            PhysicalDeviceType::INTEGRATED_GPU => 1,
            PhysicalDeviceType::VIRTUAL_GPU => 2,
            PhysicalDeviceType::CPU => 3,
            _ => 4,
        }).ok_or(format_err!("No suitable physical devices found"))?;

        let device_name = unsafe {
            CStr::from_ptr(&physical.properties.device_name[0]).to_string_lossy()
        };

        debug!("Physical device picked: {}", device_name);

        let families = config.queues.configure(&physical.queues);

        let (create_queues, get_queues): (SmallVec<[_; 32]>, SmallVec<[_; 32]>) = families.into_iter()
            .map(|(index, priorities)| {                
                let info = DeviceQueueCreateInfo::builder()
                    .queue_family_index(index.0)
                    .queue_priorities(priorities.as_ref())
                    .build();
                let get = (index, priorities.as_ref().len() as u32);
                (info, get)
            })
            .unzip();

        debug!("Queues: {:#?}", get_queues);

        let device = unsafe {
            instance.create_device(
                physical.handle,
                &DeviceCreateInfo::builder()
                    .queue_create_infos(&create_queues)
                    .enabled_extension_names(&extensions_to_enable(&physical.extensions).unwrap())
                    .enabled_features(&physical.features)
                    .build(),
                None,
            )
        }?;

        let swapchain = Swapchain::new(&instance, &device)
            .map_err(|missing| {
                format_err!("{:#?} functions are missing", missing)
            })?;

        let (types, heaps) = config.heaps.configure(&physical.memory);
        let heaps = heaps.into_iter().collect::<SmallVec<[_; 16]>>();
        let types = types.into_iter().collect::<SmallVec<[_; 32]>>();

        debug!("Heaps: {:#?}\nTypes: {:#?}", heaps, types);

        let heaps = unsafe {
            Heaps::new(types, heaps)
        };

        let families = unsafe {
            Families::from_device(&device, get_queues, &physical.queues)
        };

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

        trace!("Factory created");

        Ok(factory)
    }

    /// Creates a buffer that is managed with the specified properties.
    pub fn create_buffer(
        &mut self,
        info: BufferCreateInfo,
        align: u64,
        memory_usage: impl MemoryUsage,
    ) -> Result<Buffer, MemoryError> {
        self.resources
            .create_buffer(&self.device, &mut self.heaps, info, align, memory_usage)
    }

    /// Upload buffer data.
    pub fn upload_buffer(
        &mut self,
        _buffer: &mut Buffer,
        _offset: u64,
        _data: &[u8],
        _family: FamilyId,
        _access: AccessFlags,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    /// Creates an image that is mananged with the specified properties.
    pub fn create_image(
        &mut self,
        info: ImageCreateInfo,
        align: u64,
        memory_usage: impl MemoryUsage,
    ) -> Result<Image, MemoryError> {
        self.resources
            .create_image(&self.device, &mut self.heaps, info, align, memory_usage)
    }

    /// Create surface
    pub fn create_target(&self, window: Window, image_count: u32) -> Result<Target, Error> {
        let surface = self.native_surface.create_surface(&window)?;

        let swapchain = unsafe {
            let present_modes = self.surface.get_physical_device_surface_present_modes_khr(self.physical.handle, surface)?;
            debug!("Present modes: {:#?}", present_modes);

            let formats = self.surface.get_physical_device_surface_formats_khr(self.physical.handle, surface)?;
            debug!("Formats: {:#?}", formats);

            let capabilities = self.surface.get_physical_device_surface_capabilities_khr(self.physical.handle, surface)?;
            debug!("Capabilities: {:#?}", capabilities);

            self.swapchain.create_swapchain_khr(
                &SwapchainCreateInfoKHR::builder()
                    .surface(surface)
                    .min_image_count(max(min(image_count, capabilities.max_image_count), capabilities.min_image_count))
                    .image_format(formats[0].format)
                    .image_extent(capabilities.current_extent)
                    .image_array_layers(1)
                    .image_usage(capabilities.supported_usage_flags)
                    .present_mode(present_modes[0])
                    .build(),
                None,
            )
        }?;

        trace!("Target created");
        Ok(unsafe {Target::new(window, surface, swapchain)})
    }

    pub fn destroy_target(&self, target: Target) -> Window {
        unsafe {
            let (window, surface, swapchain) = target.dispose();
            self.swapchain.destroy_swapchain_khr(swapchain, None);
            self.surface.destroy_surface_khr(surface, None);
            trace!("Target destroyed");
            window
        }
    }

    /// Build a `Renderer<Self, T>` from the `RendererBuilder` and a render info
    pub fn build_render<'a, R, T>(&mut self, builder: RendererBuilder<R>, data: &mut T) -> Result<R::Renderer, Error>
    where
        R: RendererDesc<T>,
    {
        let image_count = builder.image_count;
        let targets = builder.windows.into_iter().map(|window| self.create_target(window, image_count)).collect::<Result<_, _>>()?;
        Ok(builder.desc.build(targets, self, data))
    }

    pub fn dispose(self) {
        self.families.dispose(&self.device);
        self.heaps.dispose(&self.device);
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }

        self.relevant.dispose();
        trace!("Factory destroyed");
    }
}

unsafe fn extension_name_cstr(e: &ExtensionProperties) -> &CStr {
    CStr::from_ptr(e.extension_name[..].as_ptr())
}

fn extensions_to_enable(available: &[ExtensionProperties]) -> Result<Vec<*const c_char>, Error> {
    let names = vec![
        Surface::name().as_ptr(),
        Swapchain::name().as_ptr(),
        NativeSurface::name().as_ptr(),
    ];

    let not_found = unsafe {
        names.iter().cloned()
            .filter_map(|name| {
                let cstr_name = CStr::from_ptr(name);
                trace!("Look for {:?}", cstr_name);
                if available.iter().find(|e| extension_name_cstr(e) == cstr_name).is_none() {
                    Some(cstr_name)
                } else {
                    None
                }
            }).collect::<Vec<_>>()
    };

    if not_found.is_empty() {
        Ok(names)
    } else {
        Err(format_err!("Extensions {:#?} are not available: {:#?}", not_found, available))
    }    
}
