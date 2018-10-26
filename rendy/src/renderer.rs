
use command::Frames;
use factory::Factory;
use target::Target;
use winit::Window;

pub trait Renderer<F, T> {

    type Desc: RendererDesc<F, T>;

    fn builder() -> RendererBuilder<Self::Desc>
    where
        Self::Desc: Default,
    {
        RendererBuilder::new(Self::Desc::default())
    }

    fn run(&mut self, factory: &mut F, data: &mut T, frames: &mut Frames);
}

pub trait RendererDesc<F, T> {
    type Renderer: Renderer<F, T>;

    fn build(self, targets: Vec<Target>, factory: &mut F, data: &mut T) -> Self::Renderer;
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct RendererBuilder<R> {
    #[derivative(Debug = "ignore")]
    pub(crate) windows: Vec<Window>,
    pub(crate) image_count: u32,
    pub(crate) desc: R,
}

impl<R> RendererBuilder<R> {
    pub fn new(desc: R) -> Self {
        RendererBuilder {
            windows: Vec::new(),
            image_count: 3,
            desc,
        }
    }

    pub fn with_window(mut self, window: Window) -> Self {
        self.windows.push(window);
        self
    }

    pub fn with_image_count(mut self, image_count: u32) -> Self {
        self.image_count = image_count;
        self
    }
}
