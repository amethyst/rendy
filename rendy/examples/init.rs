extern crate ash;
extern crate failure;
extern crate rendy;

#[macro_use]
extern crate log;
extern crate env_logger;

use ash::version::V1_0;

use rendy::{
    Factory, Config,
};
// use winit::{EventsLoop, WindowBuilder};

// use std::marker::PhantomData;

fn main() -> Result<(), failure::Error> {
    env_logger::init();

    let config: Config = Default::default();

    let factory: Factory<V1_0> = Factory::new(config)?;


    


    factory.dispose();
    Ok(())
}
