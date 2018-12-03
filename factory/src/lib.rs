
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

extern crate rendy_command as command;
extern crate rendy_memory as memory;
extern crate rendy_resource as resource;
extern crate rendy_wsi as wsi;

mod factory;
mod config;

pub use crate::{
    config::{
        BasicHeapsConfigure, Config, HeapsConfigure, OneGraphicsQueue, QueuesConfigure,
        SavedHeapsConfig, SavedQueueConfig,
    },
    factory::Factory,
};
