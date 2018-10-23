use winit::Window;

// TODO: figure out what these values should be
#[derive(Debug, Clone, Default)]
pub struct MemoryConfig {}

#[derive(Debug, Clone, Default)]
pub struct Config {
    memory: MemoryConfig,
    renders: Vec<RenderConfig>,
}

impl Config {
    pub fn new(renders: Vec<RenderConfig>) -> Self {
        Config {
            memory: MemoryConfig {},
            renders,
        }
    }
}

#[derive(Debug, Clone, Derivative)]
#[derivative(Default)]
pub struct RenderConfig {
    // #[derivative(Debug = "ignore")]
    // windows: Vec<Window>,
    #[derivative(Default(value = "3"))]
    image_count: u32,
}

impl RenderConfig {
    pub fn new() -> RenderBuilder {
        RenderBuilder::new()
    }
}

pub struct RenderBuilder {
    windows: Vec<Window>,
    image_count: u32,
}

impl RenderBuilder {
    pub fn new() -> Self {
        RenderBuilder {
            windows: Vec::new(),
            image_count: 3,
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

    pub fn build(self) -> RenderConfig {
        RenderConfig {
            //windows: self.windows,
            image_count: self.image_count,
        }
    }
}
