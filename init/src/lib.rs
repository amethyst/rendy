//!
//! Simple rendy initialization.
//! Takes most bolierplate required for init rendy on different platforms/backends.
//! It is still possible to construct everything manually if your case is not supported by this module.
//!

// #[allow(unused)]
use {
    rendy_command::Families,
    rendy_core::{
        backend_enum,
        hal::{device::CreationError, Backend, Instance as _, UnsupportedBackend},
        rendy_backend, rendy_with_dx12_backend,
        rendy_with_empty_backend, rendy_with_gl_backend, rendy_with_metal_backend,
        rendy_with_vulkan_backend, EnabledBackend, Instance,
    },
    rendy_factory::{Config, DevicesConfigure, Factory, HeapsConfigure, QueuesConfigure},
};

#[cfg(feature = "winit")]
mod windowed;

#[cfg(feature = "winit")]
pub use windowed::*;

/// Error during rendy initialization
#[derive(Clone, Debug)]
pub enum RendyInitError {
    /// Gfx creation error.
    CreationError(CreationError),

    /// Backend is unsupported.
    UnsupportedBackend(UnsupportedBackend),
}

impl From<CreationError> for RendyInitError {
    fn from(err: CreationError) -> Self {
        RendyInitError::CreationError(err)
    }
}

impl From<UnsupportedBackend> for RendyInitError {
    fn from(err: UnsupportedBackend) -> Self {
        RendyInitError::UnsupportedBackend(err)
    }
}

impl std::fmt::Display for RendyInitError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendyInitError::CreationError(err) => write!(fmt, "Cannot init rendy: {:#?}", err),
            RendyInitError::UnsupportedBackend(err) => write!(fmt, "Cannot init rendy: {:#?}", err),
        }
    }
}

impl std::error::Error for RendyInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RendyInitError::CreationError(_err) => None, // Should be `Some(err)`
            RendyInitError::UnsupportedBackend(_err) => None, // Should be `Some(err)`
        }
    }
}

/// Initialized rendy instance without window.
/// Create with `Rendy::init`.
///
/// OpenGL can't be initialized without window, see `WindowedRendy` to initialize rendy on OpenGL.
#[derive(Debug)]
pub struct Rendy<B: Backend> {
    pub families: Families<B>,
    pub factory: Factory<B>,
}

impl<B: Backend> Rendy<B> {
    pub fn init(
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
    ) -> Result<Self, RendyInitError> {
        let instance = B::Instance::create("Rendy", 1)?;
        let (factory, families) =
            rendy_factory::init_with_instance(Instance::new(instance), config)?;
        Ok(Rendy { factory, families })
    }
}

/// Error type that may be returned by `AnyRendy::init_auto`
pub struct RendyAutoInitError {
    pub errors: Vec<(EnabledBackend, RendyInitError)>,
}

impl std::fmt::Debug for RendyAutoInitError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, fmt)
    }
}

impl std::fmt::Display for RendyAutoInitError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if fmt.alternate() {
            if self.errors.is_empty() {
                writeln!(fmt, "No enabled backends among:")?;
                for &backend in &BASIC_PRIORITY {
                    writeln!(fmt, "  {:#}", backend)?;
                }
            } else {
                writeln!(fmt, "Initialization failed for all backends")?;
                for (backend, error) in &self.errors {
                    writeln!(fmt, "  {:#}: {:#}", backend, error)?;
                }
            }
        } else {
            if self.errors.is_empty() {
                write!(fmt, "No enabled backends among:")?;
                for &backend in &BASIC_PRIORITY {
                    write!(fmt, "  {}", backend)?;
                }
            } else {
                write!(fmt, "Initialization failed for all backends")?;
                for (backend, error) in &self.errors {
                    write!(fmt, "  {}: {}", backend, error)?;
                }
            }
        }
        Ok(())
    }
}

backend_enum! { #[derive(Debug)] pub enum AnyRendy(Rendy); }

impl AnyRendy {
    pub fn init_auto(
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
    ) -> Result<Self, RendyAutoInitError> {
        let mut errors = Vec::with_capacity(5);

        for backend in BASIC_PRIORITY
            .iter()
            .filter_map(|b| std::convert::TryInto::try_into(*b).ok())
        {
            match Self::init(backend, config) {
                Ok(rendy) => return Ok(rendy),
                Err(err) => errors.push((backend, err)),
            }
        }

        Err(RendyAutoInitError { errors })
    }

    #[rustfmt::skip]
    pub fn init(
        back: EnabledBackend,
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
    ) -> Result<Self, RendyInitError> {
        #![allow(unused_variables)]
        rendy_backend!(match (back): EnabledBackend {
            Dx12 => { Ok(AnyRendy::Dx12(Rendy::<rendy_core::dx12::Backend>::init(config)?)) }
            Empty => { Ok(AnyRendy::Empty(Rendy::<rendy_core::empty::Backend>::init(config)?)) }
            Gl => { Ok(AnyRendy::Gl(Rendy::<rendy_core::gl::Backend>::init(config)?)) }
            Metal => { Ok(AnyRendy::Metal(Rendy::<rendy_core::metal::Backend>::init(config)?)) }
            Vulkan => { Ok(AnyRendy::Vulkan(Rendy::<rendy_core::vulkan::Backend>::init(config)?)) }
        })
    }
}

/// Get available backends
pub fn available_backends() -> smallvec::SmallVec<[EnabledBackend; 5]> {
    #[allow(unused_mut)]
    let mut backends = smallvec::SmallVec::<[EnabledBackend; 5]>::new();
    rendy_with_dx12_backend!(backends.push(EnabledBackend::Dx12));
    rendy_with_empty_backend!(backends.push(EnabledBackend::Empty));
    rendy_with_gl_backend!(backends.push(EnabledBackend::Gl));
    rendy_with_metal_backend!(backends.push(EnabledBackend::Metal));
    rendy_with_vulkan_backend!(backends.push(EnabledBackend::Vulkan));
    backends
}

pub const BASIC_PRIORITY: [rendy_core::Backend; 4] = [
    rendy_core::Backend::Vulkan,
    rendy_core::Backend::Dx12,
    rendy_core::Backend::Metal,
    rendy_core::Backend::Gl,
];

pub fn pick_backend(
    priority: impl IntoIterator<Item = rendy_core::Backend>,
) -> Option<EnabledBackend> {
    priority
        .into_iter()
        .filter_map(|b| std::convert::TryInto::try_into(b).ok())
        .next()
}
