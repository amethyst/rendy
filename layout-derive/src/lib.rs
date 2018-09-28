extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro_derive(PipelineLayout)]
pub fn derive_pipeline_layout(input: TokenStream) -> TokenStream {
    (quote!{}).into()
}

#[proc_macro_derive(DescriptorSetLayout)]
pub fn derive_set_layout(input: TokenStream) -> TokenStream {
    (quote!{}).into()
}
