extern crate proc_macro;
extern crate syn;
extern crate quote;

use std::path::PathBuf;
use proc_macro::TokenStream;

fn out_dir() -> PathBuf {
    std::env::var("RENDY_SHADER_OUT_DIR").or(std::env::var("OUT_DIR")).unwrap().into()
}

fn glslang_validator_path() -> PathBuf {
    out_dir().join("glslang/build/bin/glslangValidator")
}

struct Input {
    name_ident: syn::Ident,
    file_lit: syn::LitStr,
}

impl syn::parse::Parse for Input {
    fn parse(stream: syn::parse::ParseStream) -> Result<Self, syn::parse::Error> {
        let name_ident = syn::Ident::parse(stream)?;
        let _ = <syn::Token!(,)>::parse(stream)?;
        let file_lit = <syn::LitStr as syn::parse::Parse>::parse(stream)?;

        Ok(Input {
            name_ident,
            file_lit,
        })
    }
}

#[proc_macro]
pub fn glsl_to_spirv(input: TokenStream) -> TokenStream {
    let Input { name_ident, file_lit } = syn::parse_macro_input!(input);

    let file = file_lit.value();
    let glsl = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(&file);
    let spv = out_dir().join(glsl.file_name().unwrap());
    let status = std::process::Command::new(glslang_validator_path())
        .arg(&glsl)
        .arg("-V")
        .arg("-o")
        .arg(&spv)
        .status()
        .unwrap();

    if !status.success() {
        panic!("glslangValidator {:?} -V -o {:?}", glsl, spv);
    }

    let glsl_code = std::fs::read_to_string(&glsl).unwrap();
    let glsl_code_lit = syn::LitStr::new(&glsl_code, file_lit.span());

    let spv_code = std::fs::read(&spv).unwrap();
    let spv_code_lit = syn::LitByteStr::new(&spv_code, file_lit.span());

    let tokens = quote::quote! {
        struct #name_ident;

        impl #name_ident {
            const NAME: &'static str = #file;
            const GLSL: &'static str = #glsl_code_lit;
            const SPIRV: &'static [u8] = #spv_code_lit;
        }
    };

    tokens.into()
}
