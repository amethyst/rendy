//! Higher-level rendy interface.

#[warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]
use rendy_command as command;
use rendy_memory as memory;
use rendy_resource as resource;
use rendy_util as util;
use rendy_wsi as wsi;

mod config;
mod factory;
mod upload;

pub use crate::{config::*, factory::*, upload::*};
