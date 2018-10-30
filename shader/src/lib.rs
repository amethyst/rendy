

extern crate rendy_shader_proc;
pub use rendy_shader_proc::compile_to_spirv_proc;

#[macro_export]
macro_rules! compile_to_spirv {
    ($(struct $name:ident { kind: $kind:ident, lang: $lang:ident, file: $file:tt, })*) => {
        $(
            $crate::compile_to_spirv_proc!($name $kind $lang $file);
        )*
    };
}
