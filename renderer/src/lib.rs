
extern crate rendy_factory as factory;

use factory::Factory;

pub trait Renderer<B: gfx_hal::Backend, T> {
    type Desc: RendererBuilder<B, T>;

    fn builder() -> Self::Desc
    where
        Self::Desc: Default,
    {
        Self::Desc::default()
    }

    fn run(&mut self, factory: &mut Factory<B>, data: &mut T);
    fn dispose(self, factory: &mut Factory<B>, data: &mut T);
}

pub trait RendererBuilder<B: gfx_hal::Backend, T> {
    type Error;
    type Renderer: Renderer<B, T>;

    fn build(self, factory: &mut Factory<B>, data: &mut T) -> Result<Self::Renderer, Self::Error>;
}
