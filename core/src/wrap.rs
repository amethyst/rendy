//! These are Vulkan Instance and Device wrappers that contain a unique ID
//! This allows checking if any other Vulkan resource belongs to a specific
//! Instance or Device. This is required to ensure we are making a safe
//! call.

use crate::hal::Backend;

#[cfg(not(feature = "no-slow-safety-checks"))]
fn new_instance_id() -> InstanceId {
    static INSTANCE_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    let id = INSTANCE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    if id == 0 {
        // Warn once.
    }

    InstanceId { id: id as u32 }
}

#[cfg(not(feature = "no-slow-safety-checks"))]
fn new_device_id(instance: InstanceId) -> DeviceId {
    static DEVICE_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    let id = DEVICE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    DeviceId {
        id: id as u32,
        instance,
    }
}

#[cfg(feature = "no-slow-safety-checks")]
fn new_instance_id() -> InstanceId {
    InstanceId {}
}

#[cfg(feature = "no-slow-safety-checks")]
fn new_device_id(instance: InstanceId) -> DeviceId {
    DeviceId { instance }
}

/// Id of the hal instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstanceId {
    /// Unique id.
    #[cfg(not(feature = "no-slow-safety-checks"))]
    pub id: u32,
}

impl InstanceId {
    /// Create new instance id.
    pub fn new() -> Self {
        new_instance_id()
    }
}

use derive_more::{Deref, DerefMut};

/// Raw instance wrapper with id.
#[derive(Deref, DerefMut)]
pub struct Instance<B: Backend> {
    #[deref]
    #[deref_mut]
    instance: B::Instance,
    id: InstanceId,
}

impl<B> Instance<B>
where
    B: Backend,
{
    /// Wrap instance value.
    pub fn new(instance: B::Instance) -> Self {
        Instance {
            id: new_instance_id(),
            instance,
        }
    }

    /// Wrap instance value.
    pub unsafe fn from_raw(instance: B::Instance, id: InstanceId) -> Self {
        Instance { id, instance }
    }

    /// Get instance id.
    pub fn id(&self) -> InstanceId {
        self.id
    }

    /// Get inner raw instance
    pub fn into_raw(self) -> B::Instance {
        self.instance
    }
}

impl<B> std::fmt::Debug for Instance<B>
where
    B: Backend,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "Instance {:?}", self.id)
    }
}

/// Id of the instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId {
    /// Unique id.
    #[cfg(not(feature = "no-slow-safety-checks"))]
    pub id: u32,

    /// Instance id.
    pub instance: InstanceId,
}

impl DeviceId {
    /// Create new device id.
    pub fn new(instance: InstanceId) -> Self {
        new_device_id(instance)
    }
}

/// Raw device wrapper with id.
#[derive(Debug, Deref, DerefMut)]
pub struct Device<B: Backend> {
    #[deref]
    #[deref_mut]
    device: B::Device,
    id: DeviceId,
}

impl<B> Device<B>
where
    B: Backend,
{
    /// Wrap device value.
    pub fn new(device: B::Device, instance: &Instance<B>) -> Self {
        Device {
            id: new_device_id(instance.id),
            device,
        }
    }

    /// Wrap device value.
    pub fn from_raw(device: B::Device, id: DeviceId) -> Self {
        Device { id, device }
    }

    /// Get device id.
    pub fn id(&self) -> DeviceId {
        self.id
    }

    /// Get inner raw device
    pub fn into_raw(self) -> B::Device {
        self.device
    }
}
