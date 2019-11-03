use std::any::TypeId;

#[cfg(all(
    feature = "dx12",
    all(target_os = "windows", not(target_arch = "wasm32"))
))]
fn dx12_backend_type_id() -> TypeId {
    TypeId::of::<crate::dx12::Backend>()
}

#[cfg(feature = "empty")]
fn empty_backend_type_id() -> TypeId {
    TypeId::of::<crate::empty::Backend>()
}

#[cfg(feature = "gl")]
fn gl_backend_type_id() -> TypeId {
    TypeId::of::<crate::gl::Backend>()
}

#[cfg(all(
    feature = "metal",
    any(
        all(not(target_arch = "wasm32"), target_os = "macos"),
        all(target_arch = "aarch64", target_os = "ios")
    )
))]
fn metal_backend_type_id() -> TypeId {
    TypeId::of::<crate::metal::Backend>()
}

#[cfg(all(
    feature = "vulkan",
    any(all(
        any(
            target_os = "windows",
            all(unix, not(any(target_os = "macos", target_os = "ios")))
        ),
        not(target_arch = "wasm32")
    ))
))]
fn vulkan_backend_type_id() -> TypeId {
    TypeId::of::<crate::vulkan::Backend>()
}

/// Backend enumerator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnabledBackend {
    /// Dx12 backend.
    #[cfg(all(
        feature = "dx12",
        all(target_os = "windows", not(target_arch = "wasm32"))
    ))]
    Dx12,

    /// Empty mock backend.
    #[cfg(feature = "empty")]
    Empty,

    /// OpenGL backend.
    #[cfg(feature = "gl")]
    Gl,

    /// Metal backend.
    #[cfg(all(
        feature = "metal",
        any(
            all(not(target_arch = "wasm32"), target_os = "macos"),
            all(target_arch = "aarch64", target_os = "ios")
        )
    ))]
    Metal,

    /// Vulkan backend.
    #[cfg(all(
        feature = "vulkan",
        all(
            any(
                target_os = "windows",
                all(unix, not(any(target_os = "macos", target_os = "ios")))
            ),
            not(target_arch = "wasm32")
        )
    ))]
    Vulkan,
}

impl EnabledBackend {
    /// Check which backend is it.
    pub fn which<B: crate::hal::Backend>() -> Self {
        #[allow(unused_variables)]
        let tid = TypeId::of::<B>();
        match () {
            #[cfg(all(
                feature = "dx12",
                all(target_os = "windows", not(target_arch = "wasm32"))
            ))]
            _ if tid == dx12_backend_type_id() => EnabledBackend::Dx12,
            #[cfg(feature = "empty")]
            _ if tid == empty_backend_type_id() => EnabledBackend::Empty,
            #[cfg(feature = "gl")]
            _ if tid == gl_backend_type_id() => EnabledBackend::Gl,
            #[cfg(all(
                feature = "metal",
                any(
                    all(not(target_arch = "wasm32"), target_os = "macos"),
                    all(target_arch = "aarch64", target_os = "ios")
                )
            ))]
            _ if tid == metal_backend_type_id() => EnabledBackend::Metal,
            #[cfg(all(
                feature = "vulkan",
                all(
                    any(
                        target_os = "windows",
                        all(unix, not(any(target_os = "macos", target_os = "ios")))
                    ),
                    not(target_arch = "wasm32")
                )
            ))]
            _ if tid == vulkan_backend_type_id() => EnabledBackend::Vulkan,
            _ => panic!("Unsupported gfx-hal backend"),
        }
    }
}

impl std::fmt::Display for EnabledBackend {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #![allow(unreachable_code)]

        fmt.write_str(match *self {
            #[cfg(all(
                feature = "dx12",
                all(target_os = "windows", not(target_arch = "wasm32"))
            ))]
            EnabledBackend::Dx12 => "dx12",
            #[cfg(feature = "empty")]
            EnabledBackend::Empty => "empty",
            #[cfg(feature = "gl")]
            EnabledBackend::Gl => "gl",
            #[cfg(all(
                feature = "metal",
                any(
                    all(not(target_arch = "wasm32"), target_os = "macos"),
                    all(target_arch = "aarch64", target_os = "ios")
                )
            ))]
            EnabledBackend::Metal => "metal",
            #[cfg(all(
                feature = "vulkan",
                all(
                    any(
                        target_os = "windows",
                        all(unix, not(any(target_os = "macos", target_os = "ios")))
                    ),
                    not(target_arch = "wasm32")
                )
            ))]
            EnabledBackend::Vulkan => "vulkan",
        })
    }
}

/// Backend enumerator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Backend {
    /// Microsoft's DirectX 12 (tm) backend
    Dx12,

    /// Empty backend. Most functions are `unimplemented!()`
    Empty,

    /// Khronos' OpenGL and WebGL backends.
    Gl,

    /// Apple's Metal (tm) backend.
    Metal,

    /// Khronos' Vulkan backend.
    Vulkan,
}

impl Backend {
    /// Check which backend is it.
    pub fn which<B: crate::hal::Backend>() -> Self {
        EnabledBackend::which::<B>().into()
    }
}

impl std::fmt::Display for Backend {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(match self {
            Backend::Dx12 => "dx12",
            Backend::Empty => "empty",
            Backend::Gl => "gl",
            Backend::Metal => "metal",
            Backend::Vulkan => "vulkan",
        })
    }
}

impl From<EnabledBackend> for Backend {
    fn from(back: EnabledBackend) -> Self {
        match back {
            #[cfg(all(
                feature = "dx12",
                all(target_os = "windows", not(target_arch = "wasm32"))
            ))]
            EnabledBackend::Dx12 => Backend::Dx12,
            #[cfg(feature = "empty")]
            EnabledBackend::Empty => Backend::Empty,
            #[cfg(feature = "gl")]
            EnabledBackend::Gl => Backend::Gl,
            #[cfg(all(
                feature = "metal",
                any(
                    all(not(target_arch = "wasm32"), target_os = "macos"),
                    all(target_arch = "aarch64", target_os = "ios")
                )
            ))]
            EnabledBackend::Metal => Backend::Metal,
            #[cfg(all(
                feature = "vulkan",
                all(
                    any(
                        target_os = "windows",
                        all(unix, not(any(target_os = "macos", target_os = "ios")))
                    ),
                    not(target_arch = "wasm32")
                )
            ))]
            EnabledBackend::Vulkan => Backend::Vulkan,
        }
    }
}

/// Unknown backend errors.
#[derive(Clone, Debug)]
pub struct ParseBackendError(String);

impl std::fmt::Display for ParseBackendError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "Unknown backend \"{}\"", self.0)
    }
}

impl std::error::Error for ParseBackendError {}

impl std::str::FromStr for Backend {
    type Err = ParseBackendError;

    fn from_str(string: &str) -> Result<Self, ParseBackendError> {
        match string {
            "Dx12" | "dx12" | "DirectX 12" => Ok(Backend::Dx12),
            "Empty" | "empty" => Ok(Backend::Empty),
            "Gl" | "gl" => Ok(Backend::Gl),
            "Metal" | "metal" => Ok(Backend::Metal),
            "Vulkan" | "vulkan" => Ok(Backend::Vulkan),
            _ => Err(ParseBackendError(string.to_string())),
        }
    }
}

/// Error signaling that particular backend is not enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NotEnabled(pub Backend);

impl std::fmt::Display for NotEnabled {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "Backend \"{}\" is not enabled", self.0)
    }
}

impl std::error::Error for NotEnabled {}

impl std::convert::TryFrom<Backend> for EnabledBackend {
    type Error = NotEnabled;

    fn try_from(back: Backend) -> Result<Self, NotEnabled> {
        match back {
            #[cfg(all(
                feature = "dx12",
                all(target_os = "windows", not(target_arch = "wasm32"))
            ))]
            Backend::Dx12 => Ok(EnabledBackend::Dx12),
            #[cfg(feature = "empty")]
            Backend::Empty => Ok(EnabledBackend::Empty),
            #[cfg(feature = "gl")]
            Backend::Gl => Ok(EnabledBackend::Gl),
            #[cfg(all(
                feature = "metal",
                any(all(
                    target_os = "macos",
                    not(target_arch = "wasm32"),
                    all(target_arch = "aarch64", target_os = "ios")
                ))
            ))]
            Backend::Metal => Ok(EnabledBackend::Metal),
            #[cfg(all(
                feature = "vulkan",
                all(
                    any(
                        target_os = "windows",
                        all(unix, not(any(target_os = "macos", target_os = "ios")))
                    ),
                    not(target_arch = "wasm32")
                )
            ))]
            Backend::Vulkan => Ok(EnabledBackend::Vulkan),
            not_enabled => Err(NotEnabled(not_enabled)),
        }
    }
}

#[doc(hidden)]
pub trait BackendSwitch {
    type Dx12;
    type Empty;
    type Gl;
    type Metal;
    type Vulkan;
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Never {}

/// Allows create new enumerations with variants for each active backend.
/// `rendy_backend!` can be used to match over it.
#[macro_export]
macro_rules! backend_enum {
    ($(#[$meta:meta])* pub enum $name:ident($data:ident $(<$($p:ty),*>)?);) => {
        $(#[$meta])*
        pub enum $name {
            Dx12(<Self as $crate::BackendSwitch>::Dx12),
            Empty(<Self as $crate::BackendSwitch>::Empty),
            Gl(<Self as $crate::BackendSwitch>::Gl),
            Metal(<Self as $crate::BackendSwitch>::Metal),
            Vulkan(<Self as $crate::BackendSwitch>::Vulkan),
        }

        impl $name {
            $crate::rendy_with_dx12_backend! {
                pub fn dx12(value: $data<$crate::dx12::Backend $($(, $p)*)?>) -> Self {
                    $name::Dx12(value)
                }
            }
            $crate::rendy_with_empty_backend! {
                pub fn empty(value: $data<$crate::empty::Backend $($(, $p)*)?>) -> Self {
                    $name::Empty(value)
                }
            }
            $crate::rendy_with_gl_backend! {
                pub fn gl(value: $data<$crate::gl::Backend $($(, $p)*)?>) -> Self {
                    $name::Gl(value)
                }
            }
            $crate::rendy_with_metal_backend! {
                pub fn metal(value: $data<$crate::metal::Backend $($(, $p)*)?>) -> Self {
                    $name::Metal(value)
                }
            }
            $crate::rendy_with_vulkan_backend! {
                pub fn vulkan(value: $data<$crate::vulkan::Backend $($(, $p)*)?>) -> Self {
                    $name::Vulkan(value)
                }
            }
        }

        impl $crate::BackendSwitch for $name {
            $crate::rendy_with_dx12_backend! { type Dx12 = $data<$crate::dx12::Backend $($(, $p)*)?>; }
            $crate::rendy_with_empty_backend! { type Empty = $data<$crate::empty::Backend $($(, $p)*)?>; }
            $crate::rendy_with_gl_backend! { type Gl = $data<$crate::gl::Backend $($(, $p)*)?>; }
            $crate::rendy_with_metal_backend! { type Metal = $data<$crate::metal::Backend $($(, $p)*)?>; }
            $crate::rendy_with_vulkan_backend! { type Vulkan = $data<$crate::vulkan::Backend $($(, $p)*)?>; }

            $crate::rendy_without_dx12_backend! { type Dx12 = $crate::Never; }
            $crate::rendy_without_empty_backend! { type Empty = $crate::Never; }
            $crate::rendy_without_gl_backend! { type Gl = $crate::Never; }
            $crate::rendy_without_metal_backend! { type Metal = $crate::Never; }
            $crate::rendy_without_vulkan_backend! { type Vulkan = $crate::Never; }
        }
    };
}

/// Execute arm with matching backend.
/// If particular backend is disabled
/// then its arm is stripped from compilation altogether.
#[macro_export]
macro_rules! rendy_backend {
    (match ($target:expr) : $enum_type:path {
        $(Dx12 => $dx12_code:block)?
        $(Empty => $empty_code:block)?
        $(Gl => $gl_code:block)?
        $(Metal => $metal_code:block)?
        $(Vulkan => $vulkan_code:block)?
        $($(use $back:ident;)?_ => $code:block)?
    }) => {{#[allow(unreachable_code, irrefutable_let_patterns)]
        || -> _ {
            #[allow(unused)] use $enum_type as EnumType;
            $($crate::rendy_with_dx12_backend!(if let EnumType :: Dx12 = $target { return { $dx12_code }; });)?
            $($crate::rendy_with_empty_backend!(if let EnumType :: Empty = $target { return { $empty_code }; });)?
            $($crate::rendy_with_gl_backend!(if let EnumType :: Gl = $target { return { $gl_code }; });)?
            $($crate::rendy_with_metal_backend!(if let EnumType :: Metal = $target { return { $metal_code }; });)?
            $($crate::rendy_with_vulkan_backend!(if let EnumType :: Vulkan = $target { return { $vulkan_code }; });)?

            $($crate::rendy_with_dx12_backend!(if let EnumType :: Dx12 = $target { $(use $crate::dx12 as $back;)? return { $code }; });)?
            $($crate::rendy_with_empty_backend!(if let EnumType :: Empty = $target { $(use $crate::empty as $back;)? return { $code }; });)?
            $($crate::rendy_with_gl_backend!(if let EnumType :: Gl = $target { $(use $crate::gl as $back;)? return { $code }; });)?
            $($crate::rendy_with_metal_backend!(if let EnumType :: Metal = $target { $(use $crate::metal as $back;)? return { $code }; });)?
            $($crate::rendy_with_vulkan_backend!(if let EnumType :: Vulkan = $target { $(use $crate::vulkan as $back;)? return { $code }; });)?

            unreachable!()
        }()
    }};

    (match ($target:expr) : $enum_type:path {
        $(Dx12($dx12_pat:pat) => $dx12_code:block)?
        $(Empty($empty_pat:pat) => $empty_code:block)?
        $(Gl($gl_pat:pat) => $gl_code:block)?
        $(Metal($metal_pat:pat) => $metal_code:block)?
        $(Vulkan($vulkan_pat:pat) => $vulkan_code:block$)?
        $($(use $back:ident;)?_($pat:pat) => $code:block)?
    }) => {{#[allow(unreachable_code, irrefutable_let_patterns)]
        || -> _ {
            #[allow(unused)] use $enum_type as EnumType;
            $($crate::rendy_with_dx12_backend!(if let EnumType :: Dx12($dx12_pat) = $target { return { $dx12_code }; });)?
            $($crate::rendy_with_empty_backend!(if let EnumType :: Empty($empty_pat) = $target { return { $empty_code }; });)?
            $($crate::rendy_with_gl_backend!(if let EnumType :: Gl($gl_pat) = $target { return { $gl_code }; });)?
            $($crate::rendy_with_metal_backend!(if let EnumType :: Metal($metal_pat) = $target { return { $metal_code }; });)?
            $($crate::rendy_with_vulkan_backend!(if let EnumType :: Vulkan($vulkan_pat) = $target { return { $vulkan_code }; });)?

            $($crate::rendy_with_dx12_backend!(if let EnumType :: Dx12($pat) = $target { $(use $crate::dx12 as $back;)? return { $code }; });)?
            $($crate::rendy_with_empty_backend!(if let EnumType :: Empty($pat) = $target { $(use $crate::empty as $back;)? return { $code }; });)?
            $($crate::rendy_with_gl_backend!(if let EnumType :: Gl($pat) = $target { $(use $crate::gl as $back;)? return { $code }; });)?
            $($crate::rendy_with_metal_backend!(if let EnumType :: Metal($pat) = $target { $(use $crate::metal as $back;)? return { $code }; });)?
            $($crate::rendy_with_vulkan_backend!(if let EnumType :: Vulkan($pat) = $target { $(use $crate::vulkan as $back;)? return { $code }; });)?

            unreachable!()
        }()
    }};
}

/// Check if specified backend would use pipeline barriers or using them is futile.
/// Piece of internal knowledge.
#[inline]
#[rustfmt::skip]
pub fn uses_pipeline_barriers<B: crate::hal::Backend>(_device: &B::Device) -> bool {
    rendy_backend!(match (EnabledBackend::which::<B>()): EnabledBackend {
        Gl => { false }
        Metal => { false }
        _ => { true }
    })
}
