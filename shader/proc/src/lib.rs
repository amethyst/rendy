//! Enable compile time shader compilation.

#![forbid(overflowing_literals)]
#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]
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
#![allow(unused_unsafe)]

extern crate proc_macro;

use proc_macro::TokenStream;
use std::path::PathBuf;

macro_rules! vk_make_version {
    ($major: expr, $minor: expr, $patch: expr) => ((($major as u32) << 22) | (($minor as u32) << 12) | $patch as u32)
}

struct Input {
    name_ident: syn::Ident,
    kind_ident: syn::Ident,
    lang_ident: syn::Ident,
    file_lit: syn::LitStr,
    entry_lit: syn::LitStr,
}

impl syn::parse::Parse for Input {
    fn parse(stream: syn::parse::ParseStream<'_>) -> Result<Self, syn::parse::Error> {
        let name_ident = syn::Ident::parse(stream)?;
        let kind_ident = syn::Ident::parse(stream)?;
        let lang_ident = syn::Ident::parse(stream)?;
        let file_lit = <syn::LitStr as syn::parse::Parse>::parse(stream)?;
        let entry_lit = <syn::LitStr as syn::parse::Parse>::parse(stream)?;

        Ok(Input {
            name_ident,
            kind_ident,
            lang_ident,
            file_lit,
            entry_lit,
        })
    }
}

fn kind(ident: &str) -> shaderc::ShaderKind {
    match ident {
        "Vertex" => shaderc::ShaderKind::Vertex,
        "Fragment" => shaderc::ShaderKind::Fragment,
        "Compute" => shaderc::ShaderKind::Compute,
        "Geometry" => shaderc::ShaderKind::Geometry,
        "TessControl" => shaderc::ShaderKind::TessControl,
        "TessEvaluation" => shaderc::ShaderKind::TessEvaluation,
        "InferFromSource" => shaderc::ShaderKind::InferFromSource,
        "DefaultVertex" => shaderc::ShaderKind::DefaultVertex,
        "DefaultFragment" => shaderc::ShaderKind::DefaultFragment,
        "DefaultCompute" => shaderc::ShaderKind::DefaultCompute,
        "DefaultGeometry" => shaderc::ShaderKind::DefaultGeometry,
        "DefaultTessControl" => shaderc::ShaderKind::DefaultTessControl,
        "DefaultTessEvaluation" => shaderc::ShaderKind::DefaultTessEvaluation,
        "SpirvAssembly" => shaderc::ShaderKind::SpirvAssembly,
        _ => panic!("Unknown shader kind"),
    }
}

fn lang(ident: &str) -> shaderc::SourceLanguage {
    match ident {
        "GLSL" => shaderc::SourceLanguage::GLSL,
        "HLSL" => shaderc::SourceLanguage::HLSL,
        _ => panic!("Unknown shader lang"),
    }
}

/// This function implements shader compilation macro.
#[proc_macro]
pub fn compile_to_spirv_proc(input: TokenStream) -> TokenStream {
    let Input {
        name_ident,
        kind_ident,
        lang_ident,
        file_lit,
        entry_lit,
    } = syn::parse_macro_input!(input);

    let file = file_lit.value();
    let file_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(&file);

    let code = std::fs::read_to_string(&file_path).unwrap();
    let code_lit = syn::LitStr::new(&code, file_lit.span());

    let entry = entry_lit.value();

    let spirv = shaderc::Compiler::new()
        .unwrap()
        .compile_into_spirv(
            &code,
            kind(&kind_ident.to_string()),
            &file_path.to_string_lossy(),
            &entry,
            Some({
                let mut ops = shaderc::CompileOptions::new().unwrap();
                ops.set_target_env(shaderc::TargetEnv::Vulkan, vk_make_version!(1, 0, 0));
                ops.set_source_language(lang(&lang_ident.to_string()));
                ops.set_optimization_level(shaderc::OptimizationLevel::Performance);
                ops
            }).as_ref(),
        ).unwrap();

    let spirv_code = spirv.as_binary_u8();
    let spirv_code_lit = syn::LitByteStr::new(spirv_code, file_lit.span());

    let tokens = quote::quote! {
        struct #name_ident;

        impl #name_ident {
            const FILE: &'static str = #file;
            const CODE: &'static str = #code_lit;
            const SPIRV: &'static [u8] = #spirv_code_lit;
            const ENTRY: &'static [u8] = #entry_lit;
        }
    };

    tokens.into()
}
