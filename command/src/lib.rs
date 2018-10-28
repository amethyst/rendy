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

mod family;
mod buffer;
mod capability;
mod encoder;
mod pool;

pub use crate::{
    buffer::{
        Buffer,
        Submit,
        InitialState,
        RecordingState,
        ExecutableState,
        PendingState,
        InvalidState,
        IndividualReset,
        Level,
        PrimaryLevel,
        SecondaryLevel,
        Droppable,
        Resettable,
        OneShot,
        MultiShot,
        SimultaneousUse,
        RenderPassContinue,
        Usage,
    },
    capability::Capability,
    encoder::Encoder,
    family::{Family, FamilyId, Families},
    pool::{Pool, OwningPool},
};





