//!
//! Basic example initializes core type of the rendy - `Factory` and exits.
//!

use rendy::{factory::Config, init::AnyWindowedRendy};

fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("init", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let event_loop = rendy::init::winit::event_loop::EventLoop::new();
    let window = rendy::init::winit::window::WindowBuilder::new().with_title("Rendy example");

    let rendy = AnyWindowedRendy::init_auto(&config, window, &event_loop).unwrap();

    rendy::with_any_windowed_rendy!((rendy) (_, _, _, _) => {});
}
