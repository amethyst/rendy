/// Resolve into input AST if dx12 backend is enabled.
#[macro_export]
#[cfg(all(feature = "dx12", target_os = "windows", not(target_arch = "wasm32")))]
macro_rules! rendy_with_dx12_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if dx12 backend is enabled.
#[macro_export]
#[cfg(not(all(feature = "dx12", target_os = "windows", not(target_arch = "wasm32"))))]
macro_rules! rendy_with_dx12_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if dx12 backend is disabled.
#[macro_export]
#[cfg(not(all(feature = "dx12", target_os = "windows", not(target_arch = "wasm32"))))]
macro_rules! rendy_without_dx12_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if dx12 backend is disabled.
#[macro_export]
#[cfg(all(feature = "dx12", target_os = "windows", not(target_arch = "wasm32")))]
macro_rules! rendy_without_dx12_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if empty backend is enabled.
#[macro_export]
#[cfg(feature = "empty")]
macro_rules! rendy_with_empty_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if empty backend is enabled.
#[macro_export]
#[cfg(not(feature = "empty"))]
macro_rules! rendy_with_empty_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if empty backend is disabled.
#[macro_export]
#[cfg(not(feature = "empty"))]
macro_rules! rendy_without_empty_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if empty backend is disabled.
#[macro_export]
#[cfg(feature = "empty")]
macro_rules! rendy_without_empty_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if gl backend is enabled.
#[macro_export]
#[cfg(feature = "gl")]
macro_rules! rendy_with_gl_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if gl backend is enabled.
#[macro_export]
#[cfg(not(feature = "gl"))]
macro_rules! rendy_with_gl_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if gl backend is disabled.
#[macro_export]
#[cfg(not(feature = "gl"))]
macro_rules! rendy_without_gl_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if gl backend is disabled.
#[macro_export]
#[cfg(feature = "gl")]
macro_rules! rendy_without_gl_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if metal backend is enabled.
#[macro_export]
#[cfg(all(
    feature = "metal",
    any(
        all(not(target_arch = "wasm32"), target_os = "macos"),
        all(target_arch = "aarch64", target_os = "ios")
    )
))]
macro_rules! rendy_with_metal_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if metal backend is enabled.
#[macro_export]
#[cfg(not(all(
    feature = "metal",
    any(
        all(not(target_arch = "wasm32"), target_os = "macos"),
        all(target_arch = "aarch64", target_os = "ios")
    )
)))]
macro_rules! rendy_with_metal_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if metal backend is disabled.
#[macro_export]
#[cfg(not(all(
    feature = "metal",
    any(
        all(not(target_arch = "wasm32"), target_os = "macos"),
        all(target_arch = "aarch64", target_os = "ios")
    )
)))]
macro_rules! rendy_without_metal_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if metal backend is disabled.
#[macro_export]
#[cfg(all(
    feature = "metal",
    any(
        all(not(target_arch = "wasm32"), target_os = "macos"),
        all(target_arch = "aarch64", target_os = "ios")
    )
))]
macro_rules! rendy_without_metal_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if vulkan backend is enabled.
#[macro_export]
#[cfg(all(
    feature = "vulkan",
    any(
        target_os = "windows",
        all(unix, not(any(target_os = "macos", target_os = "ios")))
    ),
    not(target_arch = "wasm32")
))]
macro_rules! rendy_with_vulkan_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if vulkan backend is enabled.
#[macro_export]
#[cfg(not(all(
    feature = "vulkan",
    any(
        target_os = "windows",
        all(unix, not(any(target_os = "macos", target_os = "ios")))
    ),
    not(target_arch = "wasm32")
)))]
macro_rules! rendy_with_vulkan_backend {
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if vulkan backend is disabled.
#[macro_export]
#[cfg(not(all(
    feature = "vulkan",
    any(
        target_os = "windows",
        all(unix, not(any(target_os = "macos", target_os = "ios")))
    ),
    not(target_arch = "wasm32")
)))]
macro_rules! rendy_without_vulkan_backend {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if vulkan backend is disabled.
#[macro_export]
#[cfg(all(
    feature = "vulkan",
    any(
        target_os = "windows",
        all(unix, not(any(target_os = "macos", target_os = "ios")))
    ),
    not(target_arch = "wasm32")
))]
macro_rules! rendy_without_vulkan_backend {
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
    ($($tt:tt)*) => {};
}

/// Resolve into input AST if target architecture is not "wasm32"
#[macro_export]
#[cfg(not(target_arch = "wasm32"))]
macro_rules! rendy_not_wasm32 {
    ($($tt:tt)*) => { $($tt)* };
}

/// Resolve into input AST if rendy is requested to not perform slow safety checks.
#[macro_export]
macro_rules! rendy_without_slow_safety_checks {
    ($($tt:tt)*) => { $($tt)* };
}
