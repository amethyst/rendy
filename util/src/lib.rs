//! Crate that contains utility modules used by other rendy crates

#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

mod casts;
mod slow;
mod wrap;

pub use crate::{casts::*, slow::*, wrap::*};

/// Implement ownership checking for value with `device: DeviceId` field.
#[macro_export]
macro_rules! device_owned {
    ($type:ident<B $(, $arg:ident $(: $(?$sized:ident)* $($bound:path)|*)*)*> @ $getter:expr) => {
        #[allow(unused_qualifications)]
        impl<B $(, $arg)*> $type<B $(, $arg)*>
        where
            B: gfx_hal::Backend,
            $(
                $($arg: $(?$sized)* $($bound)*,)*
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

    ($type:ident<B $(, $arg:ident $(: $(?$sized:ident)* $($bound:path)|*)*)*>) => {
        device_owned!($type<B $(, $arg $(: $(?$sized)* $($bound)|*)*)*> @ (|s: &Self| {s.device}));
    };
}

/// Implement ownership checking for value with `instance: InstanceId` field.
#[macro_export]
macro_rules! instance_owned {
    ($type:ident<B $(, $arg:ident)*>) => {
        #[allow(unused_qualifications)]
        impl<B $(, $arg)*> $type<B $(, $arg)*>
        where
            B: gfx_hal::Backend,
            $(
                $($arg: $bound)*,
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
