//! These are Vulkan Instance and Device wrappers that contain a unique ID
//! This allows checking if any other Vulkan resource belongs to a specific
//! Instance or Device. This is required to ensure we are making a safe
//! call.

use crate::hal::Backend;

fn new_device_id(instance: InstanceId) -> DeviceId {
    DeviceId { instance }
}

/// Id of the hal instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstanceId;

use derivative::Derivative;
use derive_more::{Deref, DerefMut};

/// Raw instance wrapper with id.
#[derive(Deref, DerefMut, Derivative)]
#[derivative(Debug)]
pub struct Instance<B: Backend> {
    #[deref]
    #[deref_mut]
    #[derivative(Debug = "ignore")]
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
            id: InstanceId,
            instance,
        }
    }

    /// Wrap instance value.
    pub fn from_raw(instance: B::Instance, id: InstanceId) -> Self {
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

/// Id of the instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId {
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
