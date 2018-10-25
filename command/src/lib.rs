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

extern crate ash;

#[macro_use]
extern crate failure;
extern crate relevant;
extern crate smallvec;

mod error;
mod family;
mod frame;
mod buffer;
mod capability;
mod encoder;
mod pool;

pub use buffer::{Buffer, Submit};
pub use capability::Capability;
pub use encoder::Encoder;
pub use family::{Family, FamilyId, Families};
pub use frame::{Frame, Frames, FrameBound, FrameIndex, CompleteFrame, FrameGen};
pub use pool::{Pool, OwningPool, FramePool};

