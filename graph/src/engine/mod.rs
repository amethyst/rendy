use std::sync::Arc;

use rendy_core::hal;

pub struct Engine<B: hal::Backend> {
    device: Arc<B::Device>,

    pipeline_cache: Arc<crate::command2::PipelineCache<B>>,
}
