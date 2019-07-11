/// Resolve into input AST if empty backend is enabled.
#[macro_export]
#[cfg(feature = "gfx-backend-empty")]
macro_rules! rendy_with_empty_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if empty backend is enabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-empty"))]
macro_rules! rendy_with_empty_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if empty backend is disabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-empty"))]
macro_rules! rendy_without_empty_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if empty backend is disabled.
#[macro_export]
#[cfg(feature = "gfx-backend-empty")]
macro_rules! rendy_without_empty_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if dx12 backend is enabled.
#[macro_export]
#[cfg(feature = "gfx-backend-dx12")]
macro_rules! rendy_with_dx12_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if dx12 backend is enabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-dx12"))]
macro_rules! rendy_with_dx12_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if dx12 backend is disabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-dx12"))]
macro_rules! rendy_without_dx12_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if dx12 backend is disabled.
#[macro_export]
#[cfg(feature = "gfx-backend-dx12")]
macro_rules! rendy_without_dx12_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if gl backend is enabled.
#[macro_export]
#[cfg(feature = "gfx-backend-gl")]
macro_rules! rendy_with_gl_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if gl backend is enabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-gl"))]
macro_rules! rendy_with_gl_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if gl backend is disabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-gl"))]
macro_rules! rendy_without_gl_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if gl backend is disabled.
#[macro_export]
#[cfg(feature = "gfx-backend-gl")]
macro_rules! rendy_without_gl_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if metal backend is enabled.
#[macro_export]
#[cfg(feature = "gfx-backend-metal")]
macro_rules! rendy_with_metal_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if metal backend is enabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-metal"))]
macro_rules! rendy_with_metal_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if metal backend is disabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-metal"))]
macro_rules! rendy_without_metal_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if metal backend is disabled.
#[macro_export]
#[cfg(feature = "gfx-backend-metal")]
macro_rules! rendy_without_metal_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if vulkan backend is enabled.
#[macro_export]
#[cfg(feature = "gfx-backend-vulkan")]
macro_rules! rendy_with_vulkan_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if vulkan backend is enabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-vulkan"))]
macro_rules! rendy_with_vulkan_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if vulkan backend is disabled.
#[macro_export]
#[cfg(not(feature = "gfx-backend-vulkan"))]
macro_rules! rendy_without_vulkan_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if vulkan backend is disabled.
#[macro_export]
#[cfg(feature = "gfx-backend-vulkan")]
macro_rules! rendy_without_vulkan_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if rendy is requested to not perform slow safety checks.
#[macro_export]
#[cfg(feature = "no-slow-safety-checks")]
macro_rules! rendy_without_slow_safety_checks {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if rendy is requested to not perform slow safety checks.
#[macro_export]
#[cfg(not(feature = "no-slow-safety-checks"))]
macro_rules! rendy_without_slow_safety_checks {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if rendy is requested to perform slow safety checks.
#[macro_export]
#[cfg(not(feature = "no-slow-safety-checks"))]
macro_rules! rendy_with_slow_safety_checks {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if rendy is requested to perform slow safety checks.
#[macro_export]
#[cfg(feature = "no-slow-safety-checks")]
macro_rules! rendy_with_slow_safety_checks {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if target architecture is "wasm32"
#[macro_export]
#[cfg(target_arch = "wasm32")]
macro_rules! rendy_wasm32 {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if target architecture is "wasm32"
#[macro_export]
#[cfg(not(target_arch = "wasm32"))]
macro_rules! rendy_wasm32 {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if target architecture is not "wasm32"
#[macro_export]
#[cfg(target_arch = "wasm32")]
macro_rules! rendy_not_wasm32 {
    ($($tt:tt)*) => { };
}

/// Resolve into input AST if target architecture is not "wasm32"
#[macro_export]
#[cfg(not(target_arch = "wasm32"))]
macro_rules! rendy_not_wasm32 {
    ($($tt:tt)*) => { $($tt)* };
}


/// Execute arm with matching backend.
/// If particular backend is disabled
/// then its arm is stripped from compilation altogether.
#[macro_export]
macro_rules! rendy_backend_match {
    ($target:path {
        $(empty => $empty_code:block)?
        $(dx12 => $dx12_code:block)?
        $(gl => $gl_code:block)?
        $(metal => $metal_code:block)?
        $(vulkan => $vulkan_code:block)?
    }) => {{
        $($crate::rendy_with_empty_backend!(if std::any::TypeId::of::<$target>() == std::any::TypeId::of::<$crate::empty::Backend>() { return $empty_code; }))?;
        $($crate::rendy_with_dx12_backend!(if std::any::TypeId::of::<$target>() == std::any::TypeId::of::<$crate::dx12::Backend>() { return $dx12_code; }))?;
        $($crate::rendy_with_gl_backend!(if std::any::TypeId::of::<$target>() == std::any::TypeId::of::<$crate::gl::Backend>() { return $gl_code; }))?;
        $($crate::rendy_with_metal_backend!(if std::any::TypeId::of::<$target>() == std::any::TypeId::of::<$crate::metal::Backend>() { return $metal_code; }))?;
        $($crate::rendy_with_vulkan_backend!(if std::any::TypeId::of::<$target>() == std::any::TypeId::of::<$crate::vulkan::Backend>() { return $vulkan_code; }))?;

        panic!("
            Undefined backend requested.
            Make sure feature for required backend is enabled.
            Try to add `--features=vulkan` or if on macos `--features=metal`.
        ")
    }};

    ($target:path as $back:ident => $code:block) => {{
        $crate::rendy_backend_match!($target {
            empty => { use $crate::empty as $back; $code }
            dx12 => { use $crate::dx12 as $back; $code }
            gl => { use $crate::gl as $back; $code }
            metal => { use $crate::metal as $back; $code }
            vulkan => { use $crate::vulkan as $back; $code }
        });
    }};
}
