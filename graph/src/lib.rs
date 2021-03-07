//! Framegraph implementation for Rendy engine.

#![feature(maybe_uninit_extra, allocator_api)]

//#![warn(
//    missing_debug_implementations,
//    missing_copy_implementations,
//    missing_docs,
//    trivial_casts,
//    trivial_numeric_casts,
//    unused_extern_crates,
//    unused_import_braces,
//    unused_qualifications
//)]

use rendy_chain as chain;
use rendy_command as command;
use rendy_core as core;
use rendy_factory as factory;
use rendy_frame as frame;
use rendy_memory as memory;
use rendy_resource as resource;
use rendy_wsi as wsi;
use rendy_scheduler as scheduler;

mod shader;

mod graph_borrowable;
pub use graph_borrowable::{GraphBorrow, GraphBorrowable, DynGraphBorrow};

//mod builder;
mod exec;
mod parameter;

pub mod node;

mod engine;

mod command2;

pub mod graph;
pub use graph::Node;

use rendy_core::hal;

pub use scheduler::{
    interface::{
        GraphCtx, PassEntityCtx, EntityCtx,
    },
    resources::{
        ImageInfo, ImageMode,
    },
};
pub use hal::{
    format::Format,
};
