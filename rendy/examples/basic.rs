extern crate ash;
extern crate failure;
extern crate rendy;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate winit;

use ash::version::V1_0;

use rendy::{
    Factory, Config, BasicHeapsConfigure,
};
use winit::{EventsLoop, WindowBuilder};

// use std::marker::PhantomData;

fn main() -> Result<(), failure::Error> {
    env_logger::init();

    let config: Config = Default::default();

    let factory: Factory<V1_0> = Factory::new(config)?;

    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)?;

    event_loop.poll_events(|_| ());

    let target = factory.create_target(window, 3)?;
    factory.destroy_target(target);

    factory.dispose();
    Ok(())
}
