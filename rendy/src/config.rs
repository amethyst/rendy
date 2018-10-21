use winit::Window;

use command::FamilyId;
use queue::QueuesPicker;

// TODO: figure out what these values should be
pub struct MemoryConfig {}

pub struct Config<Q> {
    memory: MemoryConfig,
    renders: Vec<RenderConfig>,
    queue_picker: Q,
}

impl<Q> Config<Q>
where
    Q: QueuesPicker,
{
    pub fn new(renders: Vec<RenderConfig>, queue_picker: Q) -> Self {
        Config {
            memory: MemoryConfig {},
            renders,
            queue_picker,
        }
    }

    pub fn pick_queues(&self) -> Result<(FamilyId, u32), ()> {
        self.queue_picker.pick_queues()
    }
}

pub struct RenderConfig {
    windows: Vec<Window>,
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
            image_count: 0,
        }
    }

    pub fn with_window(mut self, window: Window) -> Self {
        self.windows.push(window);
        self.image_count += 1;
        self
    }

    pub fn build(mut self) -> RenderConfig {
        RenderConfig {
            windows: self.windows,
            image_count: self.image_count,
        }
    }
}
