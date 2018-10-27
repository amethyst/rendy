// TODO: module docs

#[macro_use]
pub extern crate ash;
pub extern crate rendy_command as command;
pub extern crate rendy_memory as memory;
pub extern crate rendy_resource as resource;
pub extern crate rendy_wsi as wsi;

extern crate winit;

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate log;

extern crate relevant;

#[cfg(features = "serde")]
extern crate serde;

extern crate smallvec;

mod config;
mod factory;
mod queue;
mod renderer;

pub use config::{Config, QueuesConfigure, HeapsConfigure, OneGraphicsQueue, SavedQueueConfig, BasicHeapsConfigure, SavedHeapsConfig};
pub use factory::Factory;
pub use renderer::{Renderer, RendererBuilder, RendererDesc};
