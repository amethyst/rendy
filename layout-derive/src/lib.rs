//!
//! This crates allows deriving `DescriptorSetLayout` and `PipelineLayout`
//!
//!
//!
//!
//!

#![forbid(overflowing_literals)]
#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)] // Broken.
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
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]

extern crate proc_macro;
// extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

/// Derive `PipelineLayout` for type.
#[proc_macro_derive(PipelineLayout)]
pub fn derive_pipeline_layout(_input: TokenStream) -> TokenStream {
    (quote!{}).into()
}

/// Derive `PipelineLayout` for type.
#[proc_macro_derive(DescriptorSetLayout)]
pub fn derive_set_layout(_input: TokenStream) -> TokenStream {
    (quote!{}).into()
}
