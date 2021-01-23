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
#![allow(clippy::missing_safety_doc)]

use rendy_core as core;

mod buffer;
mod capability;
mod family;
mod fence;
mod pool;

pub use crate::{buffer::*, capability::*, family::*, fence::*, pool::*};
