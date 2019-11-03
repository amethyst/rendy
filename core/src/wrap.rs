//! These are Vulkan Instance and Device wrappers that contain a unique ID
//! This allows checking if any other Vulkan resource belongs to a specific
//! Instance or Device. This is required to ensure we are making a safe
//! call.

use {
    crate::hal::Backend,
    std::ops::{Deref, DerefMut},
};

#[cfg(not(feature = "no-slow-safety-checks"))]
fn new_instance_id() -> InstanceId {
    static INSTANCE_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    let id = INSTANCE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    assert!(
        id < usize::max_value() && (id as u32) < u32::max_value(),
        "Too many instances created"
    );

    if id == 0 {
        // Warn once.
        log::info!("Slow safety checks are enabled! You can disable them in production by enabling the 'no-slow-safety-checks' feature!");
    }

    InstanceId { id: id as u32 }
}

#[cfg(not(feature = "no-slow-safety-checks"))]
fn new_device_id(instance: InstanceId) -> DeviceId {
    static DEVICE_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    let id = DEVICE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    assert!(
        id < usize::max_value() && (id as u32) < u32::max_value(),
        "Too many devices created"
    );

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

/// Raw instance wrapper with id.
pub struct Instance<B: Backend> {
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

    /// Get reference to raw instance.
    pub fn raw(&self) -> &B::Instance {
        &self.instance
    }

    /// Get mutable reference to raw instance.
    pub fn raw_mut(&mut self) -> &mut B::Instance {
        &mut self.instance
    }

    /// Get inner raw instance
    pub fn into_raw(self) -> B::Instance {
        self.instance
    }
}

impl<B> Deref for Instance<B>
where
    B: Backend,
{
    type Target = B::Instance;

    fn deref(&self) -> &B::Instance {
        self.raw()
    }
}

impl<B> DerefMut for Instance<B>
where
    B: Backend,
{
    fn deref_mut(&mut self) -> &mut B::Instance {
        self.raw_mut()
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
#[derive(Debug)]
pub struct Device<B: Backend> {
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

    /// Get reference to raw device.
    pub fn raw(&self) -> &B::Device {
        &self.device
    }

    /// Get mutable reference to raw device.
    pub fn raw_mut(&mut self) -> &mut B::Device {
        &mut self.device
    }

    /// Get inner raw device
    pub fn into_raw(self) -> B::Device {
        self.device
    }
}

impl<B> Deref for Device<B>
where
    B: Backend,
{
    type Target = B::Device;

    fn deref(&self) -> &B::Device {
        self.raw()
    }
}

/// Implement ownership checking for value with `device: DeviceId` field.
#[macro_export]
macro_rules! device_owned {
    ($type:ident<B $(, $arg:ident $(: $(?$sized:ident)? $($bound:path)?)?)*> @ $getter:expr) => {
        #[allow(unused_qualifications)]
        impl<B $(, $arg)*> $type<B $(, $arg)*>
        where
            B: $crate::hal::Backend,
            $(
                $($arg: $(?$sized)* $($bound)?,)?
            )*
        {
            /// Get owned id.
            pub fn device_id(&self) -> $crate::DeviceId {
                ($getter)(self)
            }

            /// Assert specified device is owner.
            pub fn assert_device_owner(&self, device: &$crate::Device<B>) {
                assert_eq!(self.device_id(), device.id(), "Resource is not owned by specified device");
            }

            /// Get owned id.
            pub fn instance_id(&self) -> $crate::InstanceId {
                self.device_id().instance
            }

            /// Assert specified instance is owner.
            pub fn assert_instance_owner(&self, instance: &$crate::Instance<B>) {
                assert_eq!(self.instance_id(), instance.id(), "Resource is not owned by specified instance");
            }
        }
    };

    ($type:ident<B $(, $arg:ident $(: $(?$sized:ident)? $($bound:path)?)?)*>) => {
        device_owned!($type<B $(, $arg $(: $(?$sized)? $($bound)?)?)*> @ (|s: &Self| {s.device}));
    };
}

/// Implement ownership checking for value with `instance: InstanceId` field.
#[macro_export]
macro_rules! instance_owned {
    ($type:ident<B $(, $arg:ident $(: $(?$sized:ident)? $($bound:path)?)?)*>) => {
        #[allow(unused_qualifications)]
        impl<B $(, $arg)*> $type<B $(, $arg)*>
        where
            B: $crate::hal::Backend,
            $(
                $($arg: $(?$sized)? $($bound)?,)?
            )*
        {
            /// Get owned id.
            pub fn instance_id(&self) -> $crate::InstanceId {
                self.instance
            }

            /// Assert specified instance is owner.
            pub fn assert_instance_owner(&self, instance: &Instance<B>) {
                assert_eq!(self.instance_id(), instance.id());
            }
        }
    };
}
