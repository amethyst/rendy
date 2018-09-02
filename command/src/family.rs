
pub struct Family<Q> {
    index: u32,
    queues: Vec<Q>,
}

impl<Q> Family<Q> {
    pub fn queues(&mut self) -> &mut [Q] {
        &mut self.queues
    }

    pub fn create_pool<D, T, R>(&mut self, device: &mut D) -> Pool<P, T, R> {
        unsafe {
            device.create_command_pool()
        }
    }
}
