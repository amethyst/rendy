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

//use rendy_chain as chain;
use rendy_command as command;
use rendy_core as core;
use rendy_factory as factory;
//use rendy_frame as frame;
use rendy_memory as memory;
use rendy_resource as resource;
use rendy_wsi as wsi;
use rendy_scheduler as scheduler;
use rendy_shader as shader;

mod graph_borrowable;
pub use graph_borrowable::{GraphBorrow, GraphBorrowable, DynGraphBorrow};
mod slice_buf;
pub use slice_buf::SliceBuf;

//mod builder;
mod exec;
pub use exec::GraphicsPipelineBuilder;

mod parameter;

pub mod node;

//mod engine;

mod command2;
pub use command2::{Cache, ShaderSetKey};

pub mod graph;
pub use graph::{Node, GraphConstructCtx};

mod frame;
pub use frame::{Frame, Frames};

use rendy_core::hal;

pub use scheduler::{
    interface::{
        GraphCtx, PassEntityCtx, EntityCtx, ImageId, BufferId,
    },
    resources::{
        ImageInfo, ImageMode,
    },
};
pub use hal::{
    format::Format,
};
pub use rendy_shader::ShaderId;
