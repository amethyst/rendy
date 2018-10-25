
use command::Frames;
use factory::Factory;
use winit::Window;

pub trait Render<F, T> {
    fn run(&mut self, factory: &mut F, data: &mut T, frames: &mut Frames);
}

#[derive(Derivative)]
#[derivative(Debug, Default(new = "true"))]
pub struct RenderBuilder {
    #[derivative(Debug = "ignore")]
    windows: Vec<Window>,
    #[derivative(Default(value = "3"))]
    image_count: u32,
}

impl RenderBuilder {
    pub fn with_window(mut self, window: Window) -> Self {
        self.windows.push(window);
        self
    }

    pub fn with_image_count(mut self, image_count: u32) -> Self {
        self.image_count = image_count;
        self
    }
}
