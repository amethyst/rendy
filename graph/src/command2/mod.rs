use std::sync::Arc;
use std::mem::MaybeUninit;

use rendy_core::hal;
use hal::device::Device as _;

#[derive(Debug)]
pub struct PipelineCache<B: hal::Backend> {
    pub device: Arc<B::Device>,
    pub raw: MaybeUninit<B::PipelineCache>,
}
impl<B: hal::Backend> PipelineCache<B> {

    pub fn new(device: Arc<B::Device>, data: Option<&[u8]>) -> Result<Self, hal::device::OutOfMemory> {
        let raw = unsafe { device.create_pipeline_cache(data)? };
        Ok(Self {
            device,
            raw: MaybeUninit::new(raw),
        })
    }

}
impl<B: hal::Backend> Drop for PipelineCache<B> {
    fn drop(&mut self) {
        unsafe { self.device.destroy_pipeline_cache(self.raw.assume_init_read()) }
    }
}
