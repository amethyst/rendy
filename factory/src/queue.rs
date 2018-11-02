use ash::vk;

pub struct Queue<'a> {
    pub(crate) fp: &'a vk::DeviceFnV1_0,
    pub(crate) raw: vk::Queue,
}

impl<'a> Queue<'a> {
    /// Wait queue for idle.
    pub fn wait_idle(&self) {
        let result = unsafe { self.fp.queue_wait_idle(self.raw) };
        match result {
            vk::Result::SUCCESS => (),
            result => panic!("{:#?}", result),
        }
    }

    /// Get raw handle.
    pub fn raw(&self) -> vk::Queue {
        self.raw
    }

    /// Submit to the queue.
    pub fn submit(&mut self, submits: &[vk::SubmitInfo], fence: vk::Fence) {
        let _ = unsafe {
            self.fp
                .queue_submit(self.raw, submits.len() as u32, submits.as_ptr(), fence)
        };
    }
}
