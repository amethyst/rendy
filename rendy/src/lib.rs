//! Rendy's top level crate.
//! Reexports all others.

#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

#[doc(inline)]
pub use rendy_core as core;

pub use crate::core::hal;

rendy_core::rendy_with_empty_backend! { pub use crate::core::empty; }
rendy_core::rendy_with_dx12_backend! { pub use crate::core::dx12; }
rendy_core::rendy_with_gl_backend! { pub use crate::core::gl; }
rendy_core::rendy_with_metal_backend! { pub use crate::core::metal; }
rendy_core::rendy_with_vulkan_backend! { pub use crate::core::vulkan; }

#[cfg(feature = "command")]
#[doc(inline)]
pub use rendy_command as command;

#[cfg(feature = "descriptor")]
#[doc(inline)]
pub use rendy_descriptor as descriptor;

#[cfg(feature = "factory")]
#[doc(inline)]
pub use rendy_factory as factory;

#[cfg(feature = "frame")]
#[doc(inline)]
pub use rendy_frame as frame;

#[cfg(feature = "graph")]
#[doc(inline)]
pub use rendy_graph as graph;

#[cfg(feature = "init")]
#[doc(inline)]
pub use rendy_init as init;

#[cfg(feature = "memory")]
#[doc(inline)]
pub use rendy_memory as memory;

#[cfg(feature = "mesh")]
#[doc(inline)]
pub use rendy_mesh as mesh;

#[cfg(feature = "resource")]
#[doc(inline)]
pub use rendy_resource as resource;

#[cfg(feature = "shader")]
#[doc(inline)]
pub use rendy_shader as shader;

#[cfg(feature = "texture")]
#[doc(inline)]
pub use rendy_texture as texture;

#[cfg(feature = "wsi")]
#[doc(inline)]
pub use rendy_wsi as wsi;

/// Init rendy and execute code based on chosen backend
#[cfg(feature = "init")]
#[macro_export]
macro_rules! with_any_rendy {
    (($rendy:expr) $(use $back:ident;)?($factory:pat, $families:pat) => $code:block) => {{
        $crate::core::rendy_backend!(match ($rendy): $crate::init::AnyRendy {
            $(use $back;)?_($crate::init::Rendy { factory: $factory, families: $families }) => { $code }
        })
    }}
}

/// Init rendy and execute code based on chosen backend
#[cfg(feature = "init")]
#[macro_export]
macro_rules! with_any_windowed_rendy {
    (($rendy:expr) $(use $back:ident;)? ($factory:pat, $families:pat, $surface:pat, $window:pat) => $code:block) => {{
        $crate::core::rendy_backend!(match ($rendy): $crate::init::AnyWindowedRendy {
            $(use $back;)?_($crate::init::WindowedRendy { factory: $factory, families: $families, surface: $surface, window: $window }) => { $code }
        })
    }}
}
