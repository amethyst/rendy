use command::Frame;
use device::Device;
use factory::Factory;
use winit::Window;

pub trait Render<D, T>
where
    D: Device,
{
    fn run(&mut self, data: &mut T, factory: &mut Factory<D>, frame: &mut Frame<D::Fence>);
}

pub struct Target<D, R>
where
    D: Device,
{
    surface: D::Surface,
    render: R,
    frame: Frame<D::Fence>,
}
