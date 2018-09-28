//! This crates makes it simpler to define shader interface layout
//! and fill in data and resources required.
//!

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
#![deny(rust_2018_compatibility)]
// #![deny(rust_2018_idioms)]

//!
//! #[derive(PipelineDesc)]
//! struct PipelineFoo {
//!     set_a: DescriptorSetA,
//!     set_b: DescriptorSetB,
//!     push_c: PushConstantC,
//!     push_d: PushConstantD,
//! }
//!
//! #[derive(DescriptorSetDesc)]
//! struct DescriptorSetA {
//!     descriptor_a: Sampler,
//!     descriptor_b: ImageView,
//!     descriptor_c: UniformBuffer,
//! }
//!

#[macro_use]
extern crate bitflags;

extern crate rendy_memory as memory;
extern crate rendy_resource as resource;

mod descriptor;
mod device;
mod pipeline;
mod set;
mod shaders;

pub use device::Device;
pub use shaders::ShaderStageFlags;
