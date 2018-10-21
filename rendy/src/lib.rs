// TODO: module docs

extern crate rendy_command;
extern crate rendy_memory;
extern crate rendy_resource;

#[cfg(feature = "hal")]
pub extern crate gfx_hal as hal;

#[cfg(feature = "ash")]
pub extern crate ash;

mod config;
mod device;
mod factory;
mod physical_device;
mod queue;
mod render;

pub use device::Device;
pub use factory::Factory;
pub use queue::QueuesPicker;
