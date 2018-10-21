// TODO: module docs

pub extern crate rendy_command as command;
pub extern crate rendy_memory as memory;
pub extern crate rendy_resource as resource;

extern crate winit;

#[cfg(feature = "hal")]
pub extern crate gfx_hal as hal;

#[cfg(feature = "ash")]
pub extern crate ash;

mod impls;

mod config;
mod device;
mod factory;
mod init;
mod queue;
mod render;

pub use config::{Config, MemoryConfig, RenderBuilder, RenderConfig};
pub use device::Device;
pub use factory::Factory;
pub use init::init;
pub use queue::QueuesPicker;
