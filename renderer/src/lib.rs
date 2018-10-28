#[macro_use]
extern crate derivative;
extern crate winit;

extern crate rendy_factory as factory;
extern crate rendy_frame as frame;
extern crate rendy_wsi as wsi;

use winit::Window;

use factory::Factory;
use frame::Frames;
use wsi::Target;

pub trait Renderer<T> {
    type Desc: RendererDesc<T>;

    fn builder() -> RendererBuilder<Self::Desc>
    where
        Self::Desc: Default,
    {
        RendererBuilder::new(Self::Desc::default())
    }

    fn run(&mut self, factory: &mut Factory, data: &mut T, frames: &mut Frames);
}

pub trait RendererDesc<T> {
    type Renderer: Renderer<T>;

    fn build(self, targets: Vec<Target>, factory: &mut Factory, data: &mut T) -> Self::Renderer;
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
