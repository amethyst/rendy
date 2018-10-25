
use std::{ffi::{CStr, CString}, os::raw::c_char};

use ash::{
    Device,
    Instance,
    Entry,
    LoadingError,
    version::{
        DeviceV1_0,
        EntryV1_0,
        InstanceV1_0,
        FunctionPointers
    },
    vk::{
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
    },
};

use failure::Error;

use relevant::Relevant;

use smallvec::SmallVec;

use command::Families;
use memory::{HeapsConfig, Heaps, MemoryError, Usage as MemoryUsage};
use resource::{
    buffer::Buffer,
    image::Image,
    Resources,
};
use winit::Window;

use config::{Config, HeapsConfigure, QueuesConfigure};
use render::Render;

#[derive(Debug, Fail)]
#[fail(display = "{:#?}", _0)]
pub struct EntryError(LoadingError);

#[derive(Debug)]
struct PhysicalDeviceInfo {
    handle: PhysicalDevice,
    properties: PhysicalDeviceProperties,
    memory: PhysicalDeviceMemoryProperties,
    queues: Vec<QueueFamilyProperties>,
    // ext: Vec<ExtensionProperties>,
}

/// The `Factory<D>` type represents the overall creation type for `rendy`.
pub struct Factory<V: FunctionPointers> {
    instance: Instance<V>,
    physical: PhysicalDeviceInfo,
    device: Device<V>,
    families: Families,
    heaps: Heaps,
    resources: Resources,
    relevant: Relevant,
}

impl<V> Factory<V>
where
    V: FunctionPointers,
    Device<V>: DeviceV1_0,
{
    /// Creates a new `Factory` based off of a `Config<Q, W>` with some `QueuesConfigure`
    /// from the specified `PhysicalDevice`.
    pub fn new(config: Config<impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, Error>
    where
        Entry<V>: EntryV1_0<Fp = V>,
        Instance<V>: InstanceV1_0<Fp = V>,
    {
        let entry = Entry::<V>::new().map_err(EntryError)?;

        let layers = entry.enumerate_instance_layer_properties()?;
        info!("Available layers:\n{:#?}", layers);

        let extensions = entry.enumerate_instance_extension_properties()?;
        info!("Available extensions:\n{:#?}", extensions);

        let instance = unsafe {
            entry.create_instance(
                &InstanceCreateInfo::builder()
                    .application_info(
                        &ApplicationInfo::builder()
                            .application_name(&CString::new(config.app_name)?)
                            .application_version(config.app_version)
                            .build()
                    )
                    .enabled_extension_names(&extensions_to_enable(&extensions))
                    .build(),
                None,
            )
        }?;
        trace!("Instance created");

        let physicals = unsafe {
            instance.enumerate_physical_devices()?.into_iter().map(|p| PhysicalDeviceInfo {
                handle: p,
                properties: instance.get_physical_device_properties(p),
                memory: instance.get_physical_device_memory_properties(p),
                queues: instance.get_physical_device_queue_family_properties(p),
                // ext: instance.enumerate_device_extension_properties(p).unwrap(),
            })
        }.collect::<Vec<_>>();

        if physicals.is_empty() {
            bail!("No physical devices found");
        }

        info!("Physical devices:\n{:#?}", physicals);

        let physical = physicals.into_iter().min_by_key(|info| match info.properties.device_type {
            PhysicalDeviceType::DISCRETE_GPU => 0,
            PhysicalDeviceType::INTEGRATED_GPU => 1,
            PhysicalDeviceType::VIRTUAL_GPU => 2,
            PhysicalDeviceType::CPU => 3,
            _ => 4,
        }).unwrap();

        let device_name = unsafe {
            CStr::from_ptr(&physical.properties.device_name[0]).to_string_lossy()
        };

        info!("Physical device picked: {}", device_name);

        let families = config.queues.configure(&physical.queues);

        let extensions = unsafe {
            instance.enumerate_device_extension_properties(physical.handle)
        }?;

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

        info!("Queues: {:#?}", get_queues);

        let device = unsafe {
            instance.create_device(
                physical.handle,
                &DeviceCreateInfo::builder()
                    .queue_create_infos(&create_queues)
                    .enabled_extension_names(&extensions_to_enable(&extensions))
                    .build(),
                None,
            )
        }?;

        let (types, heaps) = config.heaps.configure(&physical.memory);
        let heaps = heaps.into_iter().collect::<SmallVec<[_; 16]>>();
        let types = types.into_iter().collect::<SmallVec<[_; 32]>>();

        info!("Heaps: {:#?}\nTypes: {:#?}", heaps, types);

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
            relevant: Relevant,
        };

        trace!("Factory created");

        Ok(factory)
    }

    /// Creates a buffer that is managed with the specified properties.
    pub fn create_buffer(
        &mut self,
        size: u64,
        usage: BufferUsageFlags,
        sharing_mode: SharingMode,
        align: u64,
        memory_usage: impl MemoryUsage,
    ) -> Result<Buffer, MemoryError> {
        let info = BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(sharing_mode)
            .build()
        ;

        self.resources
            .create_buffer(&self.device, &mut self.heaps, info, align, memory_usage)
    }

    /// Creates an image that is mananged with the specified properties.
    pub fn create_image(
        &mut self,
        image_type: ImageType,
        format: Format,
        extent: Extent3D,
        mip_levels: u32,
        array_layers: u32,
        samples: SampleCountFlags,
        tiling: ImageTiling,
        usage: ImageUsageFlags,
        flags: ImageCreateFlags,
        sharing_mode: SharingMode,
        align: u64,
        memory_usage: impl MemoryUsage,
        initial_layout: ImageLayout,
    ) -> Result<Image, MemoryError> {
        let info = ImageCreateInfo::builder()
            .image_type(image_type)
            .format(format)
            .extent(extent)
            .mip_levels(mip_levels)
            .array_layers(array_layers)
            .samples(samples)
            .tiling(tiling)
            .usage(usage)
            .flags(flags)
            .sharing_mode(sharing_mode)
            .initial_layout(initial_layout)
            .build()
        ;

        self.resources
            .create_image(&self.device, &mut self.heaps, info, align, memory_usage)
    }

    // pub fn create_surface<R>(window: &Window) -> Target<D, R> {
    //     unimplemented!()
    // }

    // /// Build a `Render<D, T>` from the `RenderBuilder` and a render info
    // pub fn build_render<'a, R, T>(builder: RenderBuilder, render_config: RenderConfig) -> R
    // where
    //     R: Render<D, T>,
    // {
    //     unimplemented!()
    // }

    pub fn dispose(self)
    where
        Instance<V>: InstanceV1_0<Fp = V>,
    {
        self.families.dispose(&self.device);
        self.heaps.dispose(&self.device);
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }

        self.relevant.dispose();
    }
}

fn extensions_to_enable(_: &[ExtensionProperties]) -> Vec<*const c_char> {
    const SURFACE_EXT: &'static [u8] = b"VK_KHR_surface\0";
    const SWAPCHAIN_EXT: &'static [u8] = b"VK_KHR_swapchain\0";

    #[cfg(target_os = "macos")]
    const MACOS_SURFACE_EXT: &'static [u8] = b"VK_MVK_macos_surface\0";

    let mut extensions = vec![
        SURFACE_EXT.as_ptr() as *const c_char,
        SWAPCHAIN_EXT.as_ptr() as *const c_char,
    ];

    #[cfg(target_os = "macos")]
    extensions.push(MACOS_SURFACE_EXT.as_ptr() as *const c_char);

    extensions
}
