#[macro_use]
extern crate derivative;
extern crate failure;
extern crate winit;

extern crate rendy_factory as factory;
extern crate rendy_frame as frame;
extern crate rendy_wsi as wsi;

use factory::Factory;
use frame::Frames;

pub trait Renderer<T> {
    type Desc: RendererBuilder<T>;

    fn builder() -> Self::Desc
    where
        Self::Desc: Default,
    {
        Self::Desc::default()
    }

    fn run(&mut self, factory: &mut Factory, data: &mut T);
    fn dispose(self, factory: &mut Factory, data: &mut T);
}

pub trait RendererBuilder<T> {
    type Error;
    type Renderer: Renderer<T>;

    fn build(self, factory: &mut Factory, data: &mut T) -> Result<Self::Renderer, Self::Error>;
}
