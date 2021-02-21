use std::marker::PhantomData;
use std::sync::Arc;

use rendy_core::{
    hal::{
        self,
        pso::{ShaderStageFlags, CreationError},
        device::OutOfMemory,
    },
    Device,
    hal::device::Device as DeviceTrait,
};
use rendy_resource::Extent;

use crate::{
    handle::{HasValue, HasKey},
    resource::{
        Managed,
        render_pass::{ManagedRenderPass, RenderPassCompatibilityData},
        image_view::ManagedImageView,
    },
};

pub type ManagedFramebuffer<B> = Managed<FramebufferMarker<B>>;
pub struct FramebufferMarker<B>(PhantomData<B>) where B: hal::Backend;

impl<B> HasKey for FramebufferMarker<B> where B: hal::Backend {
    type Key = Arc<FramebufferKey<B>>;
}
impl<B> HasValue for FramebufferMarker<B> where B: hal::Backend {
    type Value = ManagedFramebufferData<B>;
}

#[derive(PartialEq, Eq, Hash)]
pub struct FramebufferKey<B> where B: hal::Backend {
    pub pass: Arc<RenderPassCompatibilityData>,
    pub attachments: Vec<ManagedImageView<B>>,
    pub extent: Extent,
}

pub struct ManagedFramebufferData<B> where B: hal::Backend {
    key: Arc<FramebufferKey<B>>,
    raw: B::Framebuffer,
}

impl<B> ManagedFramebufferData<B> where B: hal::Backend {

    pub fn create(
        device: &Device<B>,
        key: Arc<FramebufferKey<B>>,
        pass: &ManagedRenderPass<B>,
    ) -> Result<Self, OutOfMemory>
    {
        let raw = unsafe {
            device.create_framebuffer(
                pass.raw(),
                key.attachments.iter().map(|v| v.raw()),
                key.extent,
            )?
        };
        let data = ManagedFramebufferData {
            key,
            raw,
        };
        Ok(data)
    }

}
