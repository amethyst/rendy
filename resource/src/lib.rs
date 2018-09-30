//! This crate provide methods to create/destroy and otherwise manage device resources.
//! Primarily focus on buffers and images.

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
#![deny(unused)]
#![deny(bad_style)]
#![deny(future_incompatible)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]


#[macro_use]
extern crate failure;

#[macro_use]
extern crate bitflags;

extern crate crossbeam_channel;
extern crate relevant;
extern crate rendy_memory as memory;

#[cfg(feature = "hal")]
extern crate gfx_hal as hal;

#[cfg(feature = "ash")]
extern crate ash;

mod device;
mod escape;
mod resources;
mod impls;
mod error;

pub mod buffer;
pub mod image;

pub use device::Device;
pub use resources::Resources;
pub use error::{ResourceError, ImageCreationError};

/// Sharing mode.
/// Resources created with sharing mode `Exclusive`
/// can be accessed only from queues of single family that owns resource.
/// Resources created with sharing mode `Concurrent` can be accessed by queues
/// from specified families.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharingMode {
    /// Sharing mode that denies sharing.
    /// Resource created with this sharing mode can be accessed
    /// only by queues of single family.
    /// This generally results in faster access than concurrent sharing mode which is not implemented yet.
    /// Ownership transfer is required in order to access resource by the queue from different family.Resources
    /// See Vulkan docs for more detail:
    /// <https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#synchronization-queue-transfers>
    Exclusive,
}

/// Memory requirements for the resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRequirements {
    /// Size of memory range required by the resource.
    pub size: u64,
    /// Minimal alignment required by the resource.
    pub align: u64,
    /// Memory type mask with bits set for memory types that support the resource.
    pub mask: u32,
}
