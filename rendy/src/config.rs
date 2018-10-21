use rendy_command::FamilyId;

use queue::QueuesPicker;
use render::RenderBuilder;

type Window = ();

// TODO: figure out what these values should be
pub struct MemoryConfig {}

pub struct Config<'a, Q> {
    memory: MemoryConfig,
    renders: Vec<RenderConfig<'a>>,
    queue_picker: Q,
}

impl<'a, Q> Config<'a, Q>
where
    Q: QueuesPicker,
{
    pub fn new(renders: Vec<RenderConfig<'a>>, queue_picker: Q) -> Self {
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

pub struct RenderConfig<'a> {
    builder: &'a RenderBuilder,
    windows: Vec<Window>,
    image_count: u32,
    // another info
}
