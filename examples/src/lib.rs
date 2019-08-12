#![cfg_attr(
    not(any(
        feature = "dx12",
        feature = "gl",
        feature = "metal",
        feature = "vulkan"
    )),
    allow(unused)
)]

use rendy::{
    command::Families,
    factory::{Config, Factory},
    graph::Graph,
    init::{AnyWindowedRendy, WindowedRendy, WithAnyWindowedRendy},
    wsi::Surface,
    core::{rendy_wasm32, rendy_not_wasm32, winit::{event::{Event, WindowEvent, StartCause}, event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder, Window}}},
    hal::{Backend, format::AsFormat as _, window::Extent2D},
};

rendy_wasm32! {
    pub use wasm_bindgen::prelude::*;
}

#[cfg(feature = "spirv-reflection")]
pub use rendy::shader::SpirvReflection;

#[cfg(not(feature = "spirv-reflection"))]
pub use rendy::mesh::AsVertex;

pub trait Example: 'static {
    fn run<B: Backend>(self, rendy: WindowedRendy<B>, event_loop: EventLoop<()>) -> !;
}

pub fn run<B, A>(mut factory: Factory<B>, mut families: Families<B>, window: Window, mut event_loop: EventLoop<()>, mut graph: Graph<B, A>, mut aux: A, mut update: impl FnMut(&mut Factory<B>, &mut A) -> bool + 'static) -> !
where
    B: Backend,
    A: 'static,
{
    // kill switch
    // std::thread::spawn(move || {
    //     while started.elapsed() < std::time::Duration::new(60, 0) {
    //         std::thread::sleep(std::time::Duration::new(1, 0));
    //     }

    //     std::process::abort();
    // });

    rendy_wasm32! {
        web_sys::window().unwrap().document().unwrap().body().unwrap().append_child(&rendy::core::winit::platform::web::WindowExtWebSys::canvas(&window));
    }

    struct Context<B: Backend, T> {
        graph: std::mem::ManuallyDrop<Graph<B, T>>,
        aux: T,
        families: Families<B>,
        factory: Factory<B>,
        window: Window,
    }

    impl<B, T> Drop for Context<B, T> where B: Backend {
        fn drop(&mut self) {
            unsafe {
                std::ptr::read(&mut*self.graph).dispose(&mut self.factory, &mut self.aux);
            }
        }
    }

    let mut context = Context {
        graph: std::mem::ManuallyDrop::new(graph),
        aux,
        families,
        factory,
        window,
    };

    let mut frames = 0u64..;

    rendy_not_wasm32! {
        let started = std::time::Instant::now();
        let mut elapsed = started.elapsed();
    }

    event_loop.run(move |event, ev, flow| {
        match event {
            Event::NewEvents(StartCause::Init) => {
                context.window.request_redraw();
            }
            Event::WindowEvent { event: WindowEvent::Destroyed, .. } | Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {}
            Event::WindowEvent { event: WindowEvent::HiDpiFactorChanged(hdi), .. } => {}
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                frames.next();
                if !update(&mut context.factory, &mut context.aux) {
                    *flow = ControlFlow::Exit;
                    return;
                }
                context.factory.maintain(&mut context.families);
                context.graph.run(&mut context.factory, &mut context.families, &context.aux);
                rendy_wasm32!{
                    context.window.request_redraw();
                }
            }
            Event::EventsCleared => {
                rendy_not_wasm32!{
                    context.window.request_redraw();
                }
            }
            Event::LoopDestroyed => {
                rendy_not_wasm32! {
                    elapsed = started.elapsed();
                    let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

                    log::info!(
                        "Render complete. FPS: {}, over {} / {:?}",
                        frames.start * 1_000_000_000 / elapsed_ns,
                        frames.start,
                        elapsed,
                    );
                }
            }
            _ => {},
        }
    })
}

pub fn start<E>(example: E) -> !
where
    E: Example,
{    
    rendy_not_wasm32! {
        env_logger::Builder::from_default_env()
            .init();
    }

    rendy_wasm32! {
        console_log::init_with_level(log::Level::Debug).unwrap();
    }

    let config: Config = Default::default();
    let window_builder = WindowBuilder::new().with_title("Rendy example").with_inner_size((800, 600).into());
    let mut event_loop = EventLoop::new();

    let mut rendy = AnyWindowedRendy::init(config, window_builder, &event_loop).unwrap();
    struct ExampleWithAnyRendy<E> {
        example: E,
        event_loop: EventLoop<()>,
    }

    enum Never {}

    impl<E> WithAnyWindowedRendy for ExampleWithAnyRendy<E>
    where
        E: Example,
    {
        type Output = Never;
        fn run<B: Backend>(self, rendy: WindowedRendy<B>) -> Never {
            self.example.run(rendy, self.event_loop)
        }
    }

    match rendy.run(ExampleWithAnyRendy { example, event_loop }) {}
}
