extern crate ash;
extern crate failure;
extern crate rendy;

extern crate env_logger;
extern crate winit;

use std::time::{Instant, Duration};
use rendy::factory::{Factory, Config};
use winit::{EventsLoop, WindowBuilder};

fn main() -> Result<(), failure::Error> {
    let started = Instant::now();

    env_logger::init();

    let config: Config = Default::default();

    let factory: Factory = Factory::new(config)?;

    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)?;

    event_loop.poll_events(|_| ());

    let target = factory.create_target(window, 3)?;

    while started.elapsed() < Duration::new(5, 0) {
        event_loop.poll_events(|_| ());
        std::thread::sleep(Duration::new(0, 1_000_000));
    };

    factory.destroy_target(target);

    factory.dispose();
    Ok(())
}
