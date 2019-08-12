//!
//! Simple rendy initialization.
//! Takes most bolierplate required for init rendy on different platforms/backends.
//! It is still possible to construct everything manually if your case is not supported by this module.
//! 

#[allow(unused)]
use {
    rendy_command::Families,
    rendy_factory::{Factory, Config, DevicesConfigure, HeapsConfigure, QueuesConfigure},
    rendy_core::{backend_enum, rendy_backend, rendy_with_gl_backend, rendy_with_dx12_backend, rendy_with_empty_backend, rendy_with_metal_backend, rendy_with_vulkan_backend, rendy_not_wasm32, rendy_wasm32, with_winit, EnabledBackend, identical_cast},
    rendy_wsi::Surface,
    rendy_core::hal::Backend,
};

with_winit! {
    use rendy_core::winit::{window::{Window, WindowBuilder}, event_loop::EventLoop};
}

/// Initialized rendy instance without window.
/// Create with `Rendy::init`.
/// 
/// OpenGL can't be initialized without window, see `WindowedRendy` to initialize rendy on OpenGL.
#[derive(Debug)]
pub struct Rendy<B: Backend> {
    pub factory: Factory<B>,
    pub families: Families<B>,
}

impl<B: Backend> Rendy<B> {
    pub fn init(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
        rendy_backend!(type B {
            Dx12 => {
                identical_cast(Rendy::init_dx12(config))
            }
            Empty => {
                identical_cast(Rendy::init_empty(config))
            }
            Gl => {
                failure::bail!("Cannot initialize OpenGL backend without window")
            }
            Metal => {
                identical_cast(Rendy::init_metal(config))
            }
            Vulkan => {
                identical_cast(Rendy::init_vulkan(config))
            }
        })
    }
}

backend_enum! { #[derive(Debug)] pub enum AnyRendy(Rendy); }

with_winit! {
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
        pub fn init<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
            rendy_backend!(type B {
                Dx12 => {
                    identical_cast(WindowedRendy::init_dx12(config, window_builder, event_loop))
                }
                Empty => {
                    identical_cast(WindowedRendy::init_empty(config, window_builder, event_loop))
                }
                Gl => {
                    identical_cast(WindowedRendy::init_gl(config, window_builder, event_loop))
                }
                Metal => {
                    identical_cast(WindowedRendy::init_metal(config, window_builder, event_loop))
                }
                Vulkan => {
                    identical_cast(WindowedRendy::init_vulkan(config, window_builder, event_loop))
                }
            })
        }
    }

    backend_enum! { #[derive(Debug)] pub enum AnyWindowedRendy(WindowedRendy); }
}

rendy_with_gl_backend! {
    /// Wrap raw GL surface.
    unsafe fn wrap_surface(factory: &Factory<rendy_core::gl::Backend>, surface: rendy_core::gl::Surface) -> Surface<rendy_core::gl::Backend> {
        Surface::from_raw(surface, factory.instance_id())
    }

    with_winit! {
        rendy_not_wasm32! {
            impl WindowedRendy<rendy_core::gl::Backend> {
                pub fn init_gl<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
                    use rendy_core::hal::format::AsFormat;

                    let windowed_context = unsafe {
                        let builder = rendy_core::gl::config_context(
                            rendy_core::gl::glutin::ContextBuilder::new(),
                            rendy_core::hal::format::Rgba8Srgb::SELF,
                            None,
                        )
                        .with_vsync(true);
                        builder.build_windowed(window_builder, event_loop)?.make_current().map_err(|(_ctx, err)| err)?
                    };
                    let (context, window) = unsafe { windowed_context.split() };
                    let surface = rendy_core::gl::Surface::from_context(context);

                    let (factory, families) = rendy_factory::init_with_instance(surface.clone(), config)?;
                    let surface = unsafe { wrap_surface(&factory, surface) };
                    Ok(WindowedRendy {factory, families, surface, window })
                }
            }
        }

        rendy_wasm32! {
            impl WindowedRendy<rendy_core::gl::Backend> {
                pub fn init_gl<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
                    let window = window_builder.build(event_loop)?;
                    let surface = rendy_core::gl::Surface::from_window(&window);

                    let (factory, families) = rendy_factory::init_with_instance(surface.clone(), config)?;
                    let surface = unsafe { wrap_surface(&factory, surface) };
                    Ok(WindowedRendy {factory, families, surface, window })
                }
            }
        }
    }
}

rendy_with_empty_backend! {
    impl Rendy<rendy_core::empty::Backend> {
        pub fn init_empty(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
            let (factory, families) = rendy_factory::init(config)?;
            Ok(Rendy {
                factory,
                families,
            })
        }
    }

    with_winit! {
        impl WindowedRendy<rendy_core::empty::Backend> {
            pub fn init_empty<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
                let mut rendy = Rendy::<rendy_core::empty::Backend>::init(config)?;
                let window = window_builder.build(event_loop)?;
                let surface = rendy.factory.create_surface_from_winit(&window);
                Ok(WindowedRendy {
                    factory: rendy.factory,
                    families: rendy.families,
                    surface,
                    window,
                })
            }
        }
    }
}

rendy_with_dx12_backend! {
    impl Rendy<rendy_core::dx12::Backend> {
        pub fn init_dx12(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
            let (factory, families) = rendy_factory::init(config)?;
            Ok(Rendy {
                factory,
                families,
            })
        }
    }

    with_winit! {
        impl WindowedRendy<rendy_core::dx12::Backend> {
            pub fn init_dx12<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
                let mut rendy = Rendy::<rendy_core::dx12::Backend>::init(config)?;
                let window = window_builder.build(event_loop)?;
                let surface = rendy.factory.create_surface_from_winit(&window);
                Ok(WindowedRendy {
                    factory: rendy.factory,
                    families: rendy.families,
                    surface,
                    window,
                })
            }
        }
    }
}

rendy_with_metal_backend! {
    impl Rendy<rendy_core::metal::Backend> {
        pub fn init_metal(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
            let (factory, families) = rendy_factory::init(config)?;
            Ok(Rendy {
                factory,
                families,
            })
        }
    }

    with_winit! {
        impl WindowedRendy<rendy_core::metal::Backend> {
            pub fn init_metal<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
                let mut rendy = Rendy::<rendy_core::metal::Backend>::init(config)?;
                let window = window_builder.build(event_loop)?;
                let surface = rendy.factory.create_surface_from_winit(&window);
                Ok(WindowedRendy {
                    factory: rendy.factory,
                    families: rendy.families,
                    surface,
                    window,
                })
            }
        }
    }
}

rendy_with_vulkan_backend! {
    impl Rendy<rendy_core::vulkan::Backend> {
        pub fn init_vulkan(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
            let (factory, families) = rendy_factory::init(config)?;
            Ok(Rendy {
                factory,
                families,
            })
        }
    }

    with_winit! {
        impl WindowedRendy<rendy_core::vulkan::Backend> {
            pub fn init_vulkan<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
                let mut rendy = Rendy::<rendy_core::vulkan::Backend>::init(config)?;
                let window = window_builder.build(event_loop)?;
                let surface = rendy.factory.create_surface_from_winit(&window);
                Ok(WindowedRendy {
                    factory: rendy.factory,
                    families: rendy.families,
                    surface,
                    window,
                })
            }
        }
    }
}


/// Get available backends
pub fn available_backends() -> impl Iterator<Item = EnabledBackend> {
    let mut backends = Vec::new();
    rendy_with_dx12_backend!(backends.push(EnabledBackend::Dx12));
    rendy_with_empty_backend!(backends.push(EnabledBackend::Empty));
    rendy_with_gl_backend!(backends.push(EnabledBackend::Gl));
    rendy_with_metal_backend!(backends.push(EnabledBackend::Metal));
    rendy_with_vulkan_backend!(backends.push(EnabledBackend::Vulkan));
    backends.into_iter()
}

pub const BASIC_PRIORITY: [rendy_core::Backend; 4] = [
    rendy_core::Backend::Vulkan,
    rendy_core::Backend::Dx12,
    rendy_core::Backend::Metal,
    rendy_core::Backend::Gl,
];

pub fn pick_backend(priority: impl Iterator<Item = rendy_core::Backend>) -> Option<EnabledBackend> {
    use std::convert::TryInto;

    for back in priority {
        if let Ok(back) = back.try_into() {
            return Some(back);
        }
    }
    None
}

pub trait WithAnyRendy {
    type Output;
    fn run<B: Backend>(self, rendy: Rendy<B>) -> Self::Output;
}

impl AnyRendy {
    pub fn init(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
        let backend = pick_backend(BASIC_PRIORITY.iter().cloned()).ok_or_else(|| failure::format_err!("Failed to pick a backend, available backends are {:#?}", available_backends().collect::<Vec<_>>()))?;
        Self::init_for(backend, config)
    }

    pub fn init_for(back: EnabledBackend, config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>) -> Result<Self, failure::Error> {
        rendy_backend!(match (back): EnabledBackend {
            Dx12 => { Ok(AnyRendy::Dx12(Rendy::<rendy_core::dx12::Backend>::init_gl(config)?)) }
            Empty => { Ok(AnyRendy::Empty(Rendy::<rendy_core::empty::Backend>::init_empty(config)?)) }
            Gl => { failure::bail!("Cannot initialize OpenGL backend without window") }
            Metal => { Ok(AnyRendy::Metal(Rendy::<rendy_core::metal::Backend>::init_metal(config)?)) }
            Vulkan => { Ok(AnyRendy::Vulkan(Rendy::<rendy_core::vulkan::Backend>::init_vulkan(config)?)) }
        })
    }

    pub fn run<T>(self, with: T) -> T::Output
    where
        T: WithAnyRendy,
    {
        rendy_backend!(match (self): AnyRendy {
            Dx12(rendy) => { with.run(rendy) }
            Empty(rendy) => { with.run(rendy) }
            Gl(rendy) => { with.run(rendy) }
            Metal(rendy) => { with.run(rendy) }
            Vulkan(rendy) => { with.run(rendy) }
        })
    }
}

with_winit! {
    pub trait WithAnyWindowedRendy {
        type Output;
        fn run<B: Backend>(self, rendy: WindowedRendy<B>) -> Self::Output;
    }

    impl AnyWindowedRendy {
        pub fn init<T: 'static>(config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
            let backend = pick_backend(BASIC_PRIORITY.iter().cloned()).ok_or_else(|| failure::format_err!("Failed to pick a backend, available backends are {:#?}", available_backends().collect::<Vec<_>>()))?;
            Self::init_for(backend, config, window_builder, event_loop)
        }

        pub fn init_for<T: 'static>(back: EnabledBackend, config: Config<impl DevicesConfigure, impl HeapsConfigure, impl QueuesConfigure>, window_builder: WindowBuilder, event_loop: &EventLoop<T>) -> Result<Self, failure::Error> {
            rendy_backend!(match (back): EnabledBackend {
                Dx12 => { Ok(AnyWindowedRendy::Dx12(WindowedRendy::<rendy_core::dx12::Backend>::init_dx12(config, window_builder, event_loop)?)) }
                Empty => { Ok(AnyWindowedRendy::Empty(WindowedRendy::<rendy_core::empty::Backend>::init_empty(config, window_builder, event_loop)?)) }
                Gl => { Ok(AnyWindowedRendy::Gl(WindowedRendy::<rendy_core::gl::Backend>::init_gl(config, window_builder, event_loop)?)) }
                Metal => { Ok(AnyWindowedRendy::Metal(WindowedRendy::<rendy_core::metal::Backend>::init_metal(config, window_builder, event_loop)?)) }
                Vulkan => { Ok(AnyWindowedRendy::Vulkan(WindowedRendy::<rendy_core::vulkan::Backend>::init_vulkan(config, window_builder, event_loop)?)) }
            })
        }

        pub fn run<T>(self, with: T) -> T::Output
        where
            T: WithAnyWindowedRendy,
        {
            rendy_backend!(match (self): AnyWindowedRendy {
                Dx12(rendy) => { with.run(rendy) }
                Empty(rendy) => { with.run(rendy) }
                Gl(rendy) => { with.run(rendy) }
                Metal(rendy) => { with.run(rendy) }
                Vulkan(rendy) => { with.run(rendy) }
            })
        }
    }
}

