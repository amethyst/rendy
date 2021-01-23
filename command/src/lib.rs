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
            B: hal::Backend,
        {
            /// Get owner id.
            pub fn family_id(&self) -> $crate::FamilyId {
                ($getter)(self)
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
            B: hal::Backend,
        {
            /// Get owner id.
            pub fn family_id(&self) -> $crate::FamilyId {
                ($getter)(self)
            }
        }
    };

    (@NOCAP $type:ident<B, $(, $args:ident)*>) => {
        family_owned!(@NOCAP $type<B, C $(, $args)*> @ |s: &Self| s.family);
    };
}

use rendy_core as core;

mod buffer;
mod capability;
mod family;
mod fence;
mod pool;

pub use crate::{buffer::*, capability::*, family::*, fence::*, pool::*};
