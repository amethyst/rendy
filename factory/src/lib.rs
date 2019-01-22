
//! Higher-level rendy interface.

#![forbid(overflowing_literals)]
#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]
#![deny(path_statements)]
#![deny(trivial_bounds)]
#![deny(type_alias_bounds)]
#![deny(unconditional_recursion)]
#![deny(unions_with_drop_fields)]
#![deny(while_true)]
#![deny(unused)]
#![deny(bad_style)]
#![deny(future_incompatible)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
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
