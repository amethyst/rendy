
use std::time::{Duration, Instant};

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

type Factory = rendy::factory::Factory<Backend>;

fn main() -> Result<(), failure::Error> {
    let started = Instant::now();

    env_logger::init();

    let config: rendy::factory::Config = Default::default();

    let factory: Factory = Factory::new(config)?;

    let mut event_loop = winit::EventsLoop::new();

    let window = winit::WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)?;

    event_loop.poll_events(|_| ());

    let target = factory.create_target(window, 3, gfx_hal::image::Usage::empty())?;

    while started.elapsed() < Duration::new(5, 0) {
        event_loop.poll_events(|_| ());
        std::thread::sleep(Duration::new(0, 1_000_000));
    }

    unsafe {
        factory.destroy_target(target);
    }

    factory.dispose();
    Ok(())
}
