//! This crate revolves around command recording and submission.

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

macro_rules! family_owned {
    ($type:ident<B, C $(, $args:ident)*> @ $getter:expr) => {
        #[allow(unused_qualifications)]
        impl<B, C $(, $args)*> $type<B, C $(, $args)*>
        where
            B: gfx_hal::Backend,
        {
            /// Get owner id.
            pub fn family_id(&self) -> $crate::FamilyId {
                ($getter)(self)
            }

            /// Assert specified family is owner.
            pub fn assert_family_owner(&self, family: &$crate::Family<B, C>) {
                assert_eq!(self.family_id(), family.id(), "Resource is not owned by specified family");
            }

            /// Assert specified device is owner.
            pub fn assert_device_owner(&self, device: &$crate::util::Device<B>) {
                assert_eq!(self.family_id().device, device.id(), "Resource is not owned by specified device");
            }

            /// Assert specified instance is owner.
            pub fn assert_instance_owner(&self, instance: &$crate::util::Instance<B>) {
                assert_eq!(self.family_id().device.instance, instance.id(), "Resource is not owned by specified instance");
            }
        }
    };

    ($type:ident<B, C $(, $args:ident)*>) => {
        family_owned!($type<B, C $(, $args)*> @ |s: &Self| s.family);
    };

    (@NOCAP $type:ident<B $(, $args:ident)*> @ $getter:expr) => {
        #[allow(unused_qualifications)]
        impl<B, $(, $args)*> $type<B, $(, $args)*>
        where
            B: gfx_hal::Backend,
        {
            /// Get owner id.
            pub fn family_id(&self) -> $crate::FamilyId {
                ($getter)(self)
            }

            /// Assert specified family is owner.
            pub fn assert_family_owner<C>(&self, family: &$crate::Family<B, C>) {
                assert_eq!(self.family_id(), family.id(), "Resource is not owned by specified family");
            }

            /// Assert specified device is owner.
            pub fn assert_device_owner(&self, device: &$crate::util::Device<B>) {
                assert_eq!(self.family_id().device, device.id(), "Resource is not owned by specified device");
            }

            /// Assert specified instance is owner.
            pub fn assert_instance_owner(&self, instance: &$crate::util::Instance<B>) {
                assert_eq!(self.family_id().device.instance, instance.id(), "Resource is not owned by specified instance");
            }
        }
    };

    (@NOCAP $type:ident<B, $(, $args:ident)*>) => {
        family_owned!(@NOCAP $type<B, C $(, $args)*> @ |s: &Self| s.family);
    };
}

use rendy_util as util;

mod buffer;
mod capability;
mod family;
mod fence;
mod pool;

pub use crate::{buffer::*, capability::*, family::*, fence::*, pool::*};
