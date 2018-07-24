

extern crate crossbeam_channel;
extern crate rendy_memory as memory;
extern crate gfx_hal as hal;
extern crate relevant;

mod escape;

pub mod buffer;
pub mod image;

#[derive(Debug)]
pub struct Resources<B: hal::Backend, T> {
    buffer: escape::Terminal<buffer::Inner<B, T>>,
    image: escape::Terminal<image::Inner<B, T>>,
}
