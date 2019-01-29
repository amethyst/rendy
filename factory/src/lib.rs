
//! Higher-level rendy interface.

#[warn(missing_debug_implementations,
       missing_copy_implementations,
       missing_docs,
       trivial_casts,
       trivial_numeric_casts,
       unused_extern_crates,
       unused_import_braces,
       unused_qualifications)]

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
