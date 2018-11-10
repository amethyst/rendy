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
#![allow(unused_unsafe)]

extern crate failure;

mod buffer;
mod capability;
mod encoder;
mod family;
mod pool;

pub use crate::{
    buffer::{
        CommandBuffer, ExecutableState, IndividualReset, InitialState, InvalidState, Level,
        MultiShot, NoIndividualReset, OneShot, PendingState, PrimaryLevel, RecordingState,
        RenderPassContinue, Resettable, SecondaryLevel, SimultaneousUse, Submit, Usage,
        CommandBufferEncoder,
    },
    capability::{Capability, Compute, General, Graphics, Transfer, Supports},
    encoder::Encoder,
    family::{families_from_device, Family},
    pool::{CommandPool, OwningCommandPool},
};
