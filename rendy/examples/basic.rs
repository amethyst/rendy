extern crate gfx_hal as hal;
extern crate rendy;
extern crate winit;

use hal::{Adapter, Backend, Instance};
use rendy::{
    command::{CapabilityFlags, Families, Family, FamilyId},
    Config, Device, Factory, QueuesPicker, RenderBuilder,
};
use winit::{EventsLoop, WindowBuilder};

use std::marker::PhantomData;

fn main() -> Result<(), ()> {
    // Create a window with winit.
    let mut events_loop = EventsLoop::new();
    let window = WindowBuilder::new()
        .with_title("Part 00: Triangle")
        .with_dimensions((848, 480).into())
        .build(&events_loop)
        .unwrap();

    let render_config = RenderBuilder::new().with_window(window).build();
    let config = Config::new(vec![render_config]);

    // TODO: migrate example to `ash`
    // let instance = backend::Instance::create("Rendy basic example", 1);

    // let adapter = instance.enumerate_adapters().remove(0);

    // type HalDevice = (
    //     <backend::Backend as Backend>::Device,
    //     PhantomData<backend::Backend>,
    // );

    //let _factory = rendy::init::<HalDevice, PickFirst, backend::Backend>(config);

    Ok(())
}

struct PickFirst;
impl QueuesPicker for PickFirst {
    fn pick_queues<Q>(
        &self,
        families: Vec<Families<Q>>,
    ) -> Result<(Family<Q, CapabilityFlags>, u32), ()> {
        unimplemented!()
    }
}
