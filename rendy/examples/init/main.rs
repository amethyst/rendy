
use rendy::{
    factory::{Config, Factory},
};

use winit::{
    EventsLoop, WindowBuilder,
};

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("init", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();

    let mut factory: Factory<Backend> = Factory::new(config).unwrap();

    factory.dispose();
}

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
fn main() {
    panic!("Specify feature: { dx12, metal, vulkan }");
}
