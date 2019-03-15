//!
//! Basic example uses the shader reflection to dump a shader and then exits.
//!
#![cfg(feature = "shader")]

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
#![allow(unused_unsafe)]

use rendy::shader::{SourceShaderInfo, ShaderKind, SourceLanguage, Shader };

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("init", log::LevelFilter::Trace)
        .init();

    let shader = SourceShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangle/shader.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );


    match shader.reflect() {
        Ok(info) => {
            println!("{:?}", info);
        },
        Err(e) => {
            panic!("Reflect on shader failed: {}", e);
        }
    }

}
