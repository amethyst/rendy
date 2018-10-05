//! Crate level docs.

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
// #![deny(unused)]
#![deny(bad_style)]
#![deny(future_incompatible)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]


#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
extern crate relevant;

extern crate rendy_chain as chain;
extern crate rendy_resource as resource;

#[cfg(feature = "hal")]
extern crate gfx_hal as hal;

#[cfg(feature = "ash")]
extern crate ash;

mod impls;

mod device;
mod error;
mod family;
mod fence;
mod frame;
mod buffer;
mod capability;
mod encoder;
mod pool;
mod queue;

pub use buffer::{Buffer, Submit};
pub use capability::{Capability, CapabilityFlags};
pub use device::{CommandBuffer, Device};
pub use encoder::Encoder;
pub use family::{Family, FamilyId, Families};
pub use fence::{FenceCreateInfo, FenceCreateFlags};
pub use frame::{Frame, FrameBound, FrameIndex, CompleteFrame, FrameGen};
pub use pool::{Pool, OwningPool, FramePool};
pub use queue::{Submission, Queue};
