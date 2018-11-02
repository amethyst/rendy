extern crate proc_macro;
extern crate quote;
extern crate shaderc;
extern crate syn;

use proc_macro::TokenStream;
use std::path::PathBuf;

struct Input {
    name_ident: syn::Ident,
    kind_ident: syn::Ident,
    lang_ident: syn::Ident,
    file_lit: syn::LitStr,
}

impl syn::parse::Parse for Input {
    fn parse(stream: syn::parse::ParseStream) -> Result<Self, syn::parse::Error> {
        let name_ident = syn::Ident::parse(stream)?;
        let kind_ident = syn::Ident::parse(stream)?;
        let lang_ident = syn::Ident::parse(stream)?;
        let file_lit = <syn::LitStr as syn::parse::Parse>::parse(stream)?;

        Ok(Input {
            name_ident,
            kind_ident,
            lang_ident,
            file_lit,
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

#[proc_macro]
pub fn compile_to_spirv_proc(input: TokenStream) -> TokenStream {
    let Input {
        name_ident,
        kind_ident,
        lang_ident,
        file_lit,
    } = syn::parse_macro_input!(input);

    let file = file_lit.value();
    let glsl = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(&file);

    let glsl_code = std::fs::read_to_string(&glsl).unwrap();
    let glsl_code_lit = syn::LitStr::new(&glsl_code, file_lit.span());

    let spirv = shaderc::Compiler::new()
        .unwrap()
        .compile_into_spirv(
            &glsl_code,
            kind(&kind_ident.to_string()),
            &glsl.to_string_lossy(),
            "main",
            Some({
                let mut ops = shaderc::CompileOptions::new().unwrap();
                ops.set_target_env(shaderc::TargetEnv::Vulkan, ash::vk_make_version!(1, 0, 0));
                ops.set_source_language(lang(&lang_ident.to_string()));
                ops
            }).as_ref(),
        ).unwrap();

    let spirv_code = spirv.as_binary_u8();
    let spirv_code_lit = syn::LitByteStr::new(spirv_code, file_lit.span());

    let tokens = quote::quote! {
        struct #name_ident;

        impl #name_ident {
            const FILE: &'static str = #file;
            const GLSL: &'static str = #glsl_code_lit;
            const SPIRV: &'static [u8] = #spirv_code_lit;
        }
    };

    tokens.into()
}
