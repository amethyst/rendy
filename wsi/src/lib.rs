pub extern crate ash;
#[macro_use] extern crate derivative;
#[macro_use] extern crate failure;

#[macro_use] extern crate log;
extern crate relevant;
#[macro_use] extern crate smallvec;
extern crate winit;

mod target;

#[cfg(target_os = "macos")]
#[macro_use] extern crate objc;

#[cfg(target_os = "macos")]
extern crate cocoa;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::NativeSurface;

pub use target::Target;
