// #![cfg_attr(
//     not(any(
//         feature = "dx12",
//         feature = "gl",
//         feature = "metal",
//         feature = "vulkan"
//     )),
//     allow(unused)
// )]

use rendy::{
    command::Families,
    factory::{Config, Factory},
    graph::Graph,
    hal::self,
    wsi::Surface,
    util::*,
};

rendy_wasm32! {
    pub use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        fn requestAnimationFrame(callback: JsValue);
    }
}

rendy_not_wasm32! {
    pub use rendy::{
        hal::format::AsFormat as _,
        wsi::winit::{EventsLoop, WindowBuilder},
    };
}

#[cfg(feature = "spirv-reflection")]
pub use rendy::shader::SpirvReflection;

#[cfg(not(feature = "spirv-reflection"))]
pub use rendy::mesh::AsVertex;

#[cfg(not(any(
    feature = "dx12",
    feature = "gl",
    feature = "metal",
    feature = "vulkan"
)))]
pub type Backend = rendy::empty::Backend;

#[cfg(feature = "dx12")]
pub type Backend = rendy::dx12::Backend;

#[cfg(feature = "gl")]
pub type Backend = rendy::gl::Backend;

#[cfg(feature = "metal")]
pub type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
pub type Backend = rendy::vulkan::Backend;

#[cfg(any(
    feature = "dx12",
    feature = "gl",
    feature = "metal",
    feature = "vulkan"
))]
pub fn run<I, U, T>(init: I)
where
    I: FnOnce(&mut Factory<Backend>, &mut Families<Backend>, Surface<Backend>) -> (Graph<Backend, T>, T, U),
    U: FnMut(&mut Factory<Backend>, &mut Families<Backend>, &mut T) -> bool + 'static,
    T: 'static,
{
    rendy_not_wasm32! {
        env_logger::Builder::from_default_env()
            .filter_module("triangle", log::LevelFilter::Trace)
            .init();
    }

    rendy_wasm32! {
        console_log::init_with_level(log::Level::Debug).unwrap();
    }

    let config: Config = Default::default();

    rendy_not_wasm32! {
        let window_builder = WindowBuilder::new().with_title("Rendy example");
        let mut events_loop = EventsLoop::new();
    }

    rendy_without_gl_backend!{
        let window = window_builder.build(&events_loop).unwrap();
        let (mut factory, mut families): (Factory<Backend>, _) = rendy::factory::init(config).unwrap();
        let surface = factory.create_surface(&window);
    }

    rendy_with_gl_backend!{
        rendy_not_wasm32! {
            let windowed_context = unsafe {
                let builder = rendy::gl::config_context(
                    rendy::gl::glutin::ContextBuilder::new(),
                    hal::format::Rgba8Srgb::SELF,
                    None,
                )
                .with_vsync(true);
                builder.build_windowed(window_builder, &events_loop)
                    .unwrap().make_current().unwrap()
            };
        }
        rendy_wasm32! {
            let window = { rendy::gl::Window };
        }
    }

    rendy_with_gl_backend!{
        rendy_wasm32! {
            let surface = rendy::gl::Surface::from_window(window);
        }

        rendy_not_wasm32! {
            let surface = rendy::gl::Surface::from_window(windowed_context);
        }
        let (mut factory, mut families) =
            rendy::factory::init_with_instance(surface.clone(), config).unwrap();
        let surface = unsafe { factory.wrap_surface(surface) };
    }

    let (mut graph, mut aux, mut update) = init(&mut factory, &mut families, surface);

    rendy_not_wasm32! {
        let mut frames = 0u64..;
        let started = std::time::Instant::now();
        let mut elapsed = started.elapsed();

        // kill switch
        // std::thread::spawn(move || {
        //     while started.elapsed() < std::time::Duration::new(60, 0) {
        //         std::thread::sleep(std::time::Duration::new(1, 0));
        //     }

        //     std::process::abort();
        // });

        for _ in &mut frames {
            if !update(&mut factory, &mut families, &mut aux) {
                break;
            }

            factory.maintain(&mut families);
            graph.run(&mut factory, &mut families, &aux);

            events_loop.poll_events(|_| ());

            elapsed = started.elapsed();
            if elapsed >= std::time::Duration::new(5, 0) { break; }
        }

        graph.dispose(&mut factory, &mut aux);
        
        let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

        log::info!(
            "Render complete. FPS: {}, over {} / {:?}",
            frames.start * 1_000_000_000 / elapsed_ns,
            frames.start,
            elapsed,
        );
    }

    rendy_wasm32! {
        Renderer {
            factory,
            families,
            graph,
            aux,
            update,
            frame: 0,
            start: None,
        }.run();
    }
}

rendy_wasm32! {
    struct Renderer<T, U> {
        factory: Factory<Backend>,
        families: Families<Backend>,
        graph: Graph<Backend, T>,
        aux: T,
        update: U,
        frame: u64,
        start: Option<f64>,
    }

    impl<T, U> Renderer<T, U>
    where
        T: 'static,
        U: FnMut(&mut Factory<Backend>, &mut Families<Backend>, &mut T) -> bool + 'static,
    {
        fn run(mut self) {
            requestAnimationFrame(Closure::once_into_js(move |timestamp: f64| {
                log::trace!("Render next frame");

                if self.start.is_none() {
                    self.start = Some(timestamp);
                }

                if !(self.update)(&mut self.factory, &mut self.families, &mut self.aux) || self.frame > 3600 {
                    self.graph.dispose(&mut self.factory, &self.aux);

                    let elapsed = timestamp - self.start.unwrap_or(timestamp);
                    let fps = self.frame * 1000 / (elapsed as u64);
                    log::info!("Render complete. FPS: {}, over {} / {}.{}", fps, self.frame, (elapsed / 1000.0) as u64, (elapsed as u64 % 1000));
                    return;
                }

                self.factory.maintain(&mut self.families);
                self.graph.run(&mut self.factory, &mut self.families, &self.aux);
                self.frame += 1;

                self.run();
            }));
        }
    }
}

#[cfg(not(any(
    feature = "dx12",
    feature = "gl",
    feature = "metal",
    feature = "vulkan"
)))]
pub fn run<T>(_: impl FnOnce(&mut Factory<Backend>, &mut Families<Backend>, Surface<Backend>) -> T) {
    panic!("Specify graphics backend via feature: { dx12, gl, metal, vulkan }");
}

#[macro_export]
#[cfg(feature = "runtime-data")]
macro_rules! get_data {
    ($path:literal) => {
        $crate::load_data($path)
    };
}

#[macro_export]
#[cfg(not(feature = "runtime-data"))]
macro_rules! get_data {
    ($path:literal) => {
        Ok::<_, std::convert::Infallible>(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/www/", $path)))
    };
}

#[cfg(feature = "runtime-data")]
pub fn load_data(path: &str) -> Result<Vec<u8>, failure::Error> {
    rendy_wasm32!{
        let err_cvt = |err| {
            failure::err_msg(js_sys::JSON::stringify(&err).map(String::from).unwrap_or_else(|_| format!("Error stringify failed")))
        };

        let req = web_sys::XmlHttpRequest::new().map_err(err_cvt)?;
        req.open_with_async("GET", path, false).map_err(err_cvt)?;
        req.send().map_err(err_cvt)?;
        let res = req.response().map_err(err_cvt)?;

        let data = js_sys::Uint8Array::from(res);
        let mut vec = vec![0; data.length() as usize];
        data.copy_to(&mut vec);

        Ok(vec)
    }

    rendy_not_wasm32! {
        let path = format!(concat!(env!("CARGO_MANIFEST_DIR"), "/www/{}"), path);
        std::fs::read(&path).map_err(|err| err.into())
    }
}

