
#[allow(unused)]
use {
    std::borrow::Cow,
    super::{Rendy, RendyInitError, pick_backend, BASIC_PRIORITY},
    rendy_command::Families,
    rendy_core::{
        hal::{Backend, Instance as _, UnsupportedBackend, device::CreationError},
        Instance,
        backend_enum, identical_cast, rendy_backend, rendy_not_wasm32, rendy_wasm32,
        rendy_with_dx12_backend, rendy_with_empty_backend, rendy_with_gl_backend,
        rendy_with_metal_backend, rendy_with_vulkan_backend, EnabledBackend,
    },
    rendy_factory::{Config, DevicesConfigure, Factory, HeapsConfigure, QueuesConfigure},
    rendy_wsi::Surface,
    winit::{
        event_loop::EventLoop,
        window::{Window, WindowBuilder},
    },
};

pub use winit;

/// Error during rendy initialization
#[derive(Debug)]
pub enum WindowedRendyInitError {
    /// Basic rendy init error.
    RendyInitError(RendyInitError),

    /// Winit error.
    Winit(winit::error::OsError),

    /// Window init error.
    WindowInitError(rendy_core::hal::window::InitError),

    Other(String),
}

impl From<RendyInitError> for WindowedRendyInitError {
    fn from(err: RendyInitError) -> Self {
        WindowedRendyInitError::RendyInitError(err)
    }
}

impl From<CreationError> for WindowedRendyInitError {
    fn from(err: CreationError) -> Self {
        WindowedRendyInitError::RendyInitError(RendyInitError::CreationError(err))
    }
}

impl From<UnsupportedBackend> for WindowedRendyInitError {
    fn from(err: UnsupportedBackend) -> Self {
        WindowedRendyInitError::RendyInitError(RendyInitError::UnsupportedBackend(err))
    }
}

impl From<winit::error::OsError> for WindowedRendyInitError {
    fn from(err: winit::error::OsError) -> Self {
        WindowedRendyInitError::Winit(err)
    }
}

impl From<rendy_core::hal::window::InitError> for WindowedRendyInitError {
    fn from(err: rendy_core::hal::window::InitError) -> Self {
        WindowedRendyInitError::WindowInitError(err)
    }
}

impl std::fmt::Display for WindowedRendyInitError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowedRendyInitError::RendyInitError(err) => write!(fmt, "Cannot init rendy: {:#?}", err),
            WindowedRendyInitError::Winit(err) => write!(fmt, "Cannot init rendy: {:#?}", err),
            WindowedRendyInitError::WindowInitError(err) => write!(fmt, "Cannot init rendy: {:#?}", err),
            WindowedRendyInitError::Other(err) => write!(fmt, "Cannot init rendy: {}", err),
        }
    }
}

impl std::error::Error for WindowedRendyInitError {}

#[derive(Debug)]
/// Initialized rendy instance with window.
/// Create with `WindowedRendy::init`.
pub struct WindowedRendy<B: Backend> {
    pub factory: Factory<B>,
    pub families: Families<B>,
    pub surface: Surface<B>,
    pub window: Window,
}

impl<B: Backend> WindowedRendy<B> {
    fn init_non_gl<T>(
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
        window_builder: Cow<WindowBuilder>,
        event_loop: &EventLoop<T>,
    ) -> Result<Self, WindowedRendyInitError> {
        let mut rendy = Rendy::<B>::init(config)?;
        let window = window_builder.into_owned().build(event_loop)?;
        let surface = rendy.factory.create_surface(&window)?;
        Ok(WindowedRendy {
            factory: rendy.factory,
            families: rendy.families,
            surface,
            window,
        })
    }
}

impl<B: Backend> WindowedRendy<B> {
    pub fn init<T: 'static>(
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
        window_builder: WindowBuilder,
        event_loop: &EventLoop<T>,
    ) -> Result<Self, WindowedRendyInitError> {
        #![allow(unreachable_code)] 

        rendy_backend!(type B {
            Gl => {
                identical_cast(WindowedRendy::init_gl(config, window_builder, event_loop))
            }
            _ => {
                Self::init_non_gl(config, Cow::Owned(window_builder), event_loop)
            }
        })
    }

    pub fn init_ref_builder<T: 'static>(
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
        window_builder: &WindowBuilder,
        event_loop: &EventLoop<T>,
    ) -> Result<Self, WindowedRendyInitError> {
        #![allow(unreachable_code)] 

        rendy_backend!(type B {
            Gl => {
                identical_cast(WindowedRendy::init_gl(config, window_builder.clone(), event_loop))
            }
            _ => {
                Self::init_non_gl(config, Cow::Borrowed(window_builder), event_loop)
            }
        })
    }
}

backend_enum! { #[derive(Debug)] pub enum AnyWindowedRendy(WindowedRendy); }

pub trait WithAnyWindowedRendy {
    type Output;
    fn run<B: Backend>(self, rendy: WindowedRendy<B>) -> Self::Output;
}

impl AnyWindowedRendy {
    pub fn init_auto<T>(
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
        window_builder: WindowBuilder,
        event_loop: &EventLoop<T>,
    ) -> Result<Self, Vec<(EnabledBackend, WindowedRendyInitError)>> {
        let mut errors = Vec::with_capacity(5);

        for backend in BASIC_PRIORITY.iter().filter_map(|b| std::convert::TryInto::try_into(*b).ok()) {
            match Self::init_ref_builder(backend, config, &window_builder, event_loop) {
                Ok(rendy) => return Ok(rendy),
                Err(err) => errors.push((backend, err)),
            }
        }

        Err(errors)
    }

    pub fn init<T>(
        back: EnabledBackend,
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
        window_builder: WindowBuilder,
        event_loop: &EventLoop<T>,
    ) -> Result<Self, WindowedRendyInitError> {
        rendy_backend!(match (back): EnabledBackend {
            Dx12 => { Ok(AnyWindowedRendy::Dx12(WindowedRendy::<rendy_core::dx12::Backend>::init(config, window_builder, event_loop)?)) }
            Empty => { Ok(AnyWindowedRendy::Empty(WindowedRendy::<rendy_core::empty::Backend>::init(config, window_builder, event_loop)?)) }
            Gl => { Ok(AnyWindowedRendy::Gl(WindowedRendy::<rendy_core::gl::Backend>::init(config, window_builder, event_loop)?)) }
            Metal => { Ok(AnyWindowedRendy::Metal(WindowedRendy::<rendy_core::metal::Backend>::init(config, window_builder, event_loop)?)) }
            Vulkan => { Ok(AnyWindowedRendy::Vulkan(WindowedRendy::<rendy_core::vulkan::Backend>::init(config, window_builder, event_loop)?)) }
        })
    }

    fn init_ref_builder<T>(
        back: EnabledBackend,
        config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>,
        window_builder: &WindowBuilder,
        event_loop: &EventLoop<T>,
    ) -> Result<Self, WindowedRendyInitError> {
        rendy_backend!(match (back): EnabledBackend {
            Dx12 => { Ok(AnyWindowedRendy::Dx12(WindowedRendy::<rendy_core::dx12::Backend>::init_ref_builder(config, window_builder, event_loop)?)) }
            Empty => { Ok(AnyWindowedRendy::Empty(WindowedRendy::<rendy_core::empty::Backend>::init_ref_builder(config, window_builder, event_loop)?)) }
            Gl => { Ok(AnyWindowedRendy::Gl(WindowedRendy::<rendy_core::gl::Backend>::init_ref_builder(config, window_builder, event_loop)?)) }
            Metal => { Ok(AnyWindowedRendy::Metal(WindowedRendy::<rendy_core::metal::Backend>::init_ref_builder(config, window_builder, event_loop)?)) }
            Vulkan => { Ok(AnyWindowedRendy::Vulkan(WindowedRendy::<rendy_core::vulkan::Backend>::init_ref_builder(config, window_builder, event_loop)?)) }
        })
    }

    pub fn run<T>(self, with: T) -> T::Output
    where
        T: WithAnyWindowedRendy,
    {
        rendy_backend!(match (self): AnyWindowedRendy {
            _(rendy) => { with.run(rendy) }
        })
    }
}

rendy_with_gl_backend! {
    /// Wrap raw GL surface.
    #[allow(unused)]
    unsafe fn wrap_surface(factory: &Factory<rendy_core::gl::Backend>, surface: rendy_core::gl::Surface) -> Surface<rendy_core::gl::Backend> {
        Surface::from_raw(surface, factory.instance_id())
    }

    rendy_not_wasm32! {
        impl WindowedRendy<rendy_core::gl::Backend> {
            pub fn init_gl<T: 'static>(config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, WindowedRendyInitError> {
                use rendy_core::hal::format::AsFormat;

                let windowed_context = unsafe {
                    let builder = rendy_core::gl::config_context(
                        rendy_core::gl::glutin::ContextBuilder::new(),
                        rendy_core::hal::format::Rgba8Srgb::SELF,
                        None,
                    )
                    .with_vsync(true);
                    builder
                        .build_windowed(window_builder, event_loop)
                        .map_err(|err| WindowedRendyInitError::Other(format!("{}", err)))?
                        .make_current()
                        .map_err(|(_ctx, err)| WindowedRendyInitError::Other(format!("{}", err)))?
                };
                let (context, window) = unsafe { windowed_context.split() };
                let surface = rendy_core::gl::Surface::from_context(context);
                let instance = Instance::new(rendy_core::gl::Instance::Surface(surface));

                let (factory, families) = rendy_factory::init_with_instance_ref(&instance, config)?;
                let surface = match rendy_core::Instance::into_raw(instance) {
                    rendy_core::gl::Instance::Surface(surface) => surface,
                    _ => unreachable!(),
                };
                let surface = unsafe { wrap_surface(&factory, surface) };
                Ok(WindowedRendy {factory, families, surface, window })
            }
        }
    }

    rendy_wasm32! {
        impl WindowedRendy<rendy_core::gl::Backend> {
            pub fn init_gl<T: 'static>(config: &Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, WindowedRendyInitError> {
                let window = window_builder.build(event_loop)?;
                let surface = rendy_core::gl::Surface::from_raw_handle(&window);
                let instance = Instance::new(surface);

                let (factory, families) = rendy_factory::init_with_instance_ref(&instance, config)?;
                let surface = unsafe { wrap_surface(&factory, instance.into_raw()) };
                Ok(WindowedRendy {factory, families, surface, window })
            }
        }
    }
}
