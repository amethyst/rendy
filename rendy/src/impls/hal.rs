use command::{Families, FamilyId};
use config::Config;
use device::Device;
use factory::Factory;
use queue::QueuesPicker;

use hal;
use winit::Window;

use std::borrow::Borrow;
use std::marker::PhantomData;

impl<B, D> Device for (D, PhantomData<B>)
where
    B: hal::Backend,
    D: Borrow<B::Device>,
{
    type Surface = B::Surface;
}

/// Initalize rendy
#[cfg(feature = "hal")]
pub fn init<D, Q, B>(config: Config, queue_picker: Q) -> Result<(Factory<D>), ()>
where
    D: Device,
    Q: QueuesPicker,
    B: BackendEx,
{
    let instance = B::init();
    unimplemented!()
}

/// Extend backend trait with initialization method and surface creation method.
pub trait BackendEx: hal::Backend {
    type Instance: hal::Instance<Backend = Self> + Send + Sync;
    fn init() -> Self::Instance;
    fn create_surface(instance: &Self::Instance, window: &Window) -> Self::Surface;
}

#[cfg(feature = "gfx-backend-vulkan")]
impl BackendEx for vulkan::Backend {
    type Instance = vulkan::Instance;
    fn init() -> Self::Instance {
        vulkan::Instance::create("gfx-render", 1)
    }
    fn create_surface(instance: &Self::Instance, window: &Window) -> Self::Surface {
        instance.create_surface(window)
    }
}

#[cfg(feature = "gfx-backend-metal")]
impl BackendEx for metal::Backend {
    type Instance = metal::Instance;
    fn init() -> Self::Instance {
        metal::Instance::create("gfx-render", 1)
    }
    fn create_surface(instance: &Self::Instance, window: &Window) -> Self::Surface {
        instance.create_surface(window)
    }
}

#[cfg(feature = "gfx-backend-dx12")]
impl BackendEx for dx12::Backend {
    type Instance = dx12::Instance;
    fn init() -> Self::Instance {
        dx12::Instance::create("gfx-render", 1)
    }
    fn create_surface(instance: &Self::Instance, window: &Window) -> Self::Surface {
        instance.create_surface(window)
    }
}
