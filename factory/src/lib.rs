
//! Higher-level rendy interface.

#![forbid(overflowing_literals)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(intra_doc_link_resolution_failure)]
#![warn(path_statements)]
#![warn(trivial_bounds)]
#![warn(type_alias_bounds)]
#![warn(unconditional_recursion)]
#![warn(unions_with_drop_fields)]
#![warn(while_true)]
#![warn(unused)]
#![warn(bad_style)]
#![warn(future_incompatible)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]
#![allow(unused_unsafe)]

use rendy_command as command;
use rendy_memory as memory;
use rendy_resource as resource;
use rendy_wsi as wsi;

mod factory;
mod config;
mod upload;

pub use crate::{
    config::{
        BasicHeapsConfigure, Config, HeapsConfigure, OneGraphicsQueue, QueuesConfigure,
        SavedHeapsConfig, SavedQueueConfig, DevicesConfigure, BasicDevicesConfigure,
    },
    factory::{Factory},
    upload::{ImageState, ImageStateOrLayout, BufferState},
};
