mod handle;

mod resource_manager;
pub use resource_manager::ResourceManager;

mod resource;
pub use resource::{
    Managed,
    buffer::ManagedBuffer,
    image::ManagedImage,
    image_view::ManagedImageView,
    shader_module::ManagedShaderModule,
    sampler::ManagedSampler,
    descriptor_set_layout::ManagedDescriptorSetLayout,
    pipeline_layout::ManagedPipelineLayout,
    render_pass::ManagedRenderPass,
    graphics_pipeline::ManagedGraphicsPipeline,
    framebuffer::ManagedFramebuffer,
};

mod dependent;
