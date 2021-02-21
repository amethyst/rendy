use {
    std::{
        sync::Arc,
        ops::Range,
        mem::ManuallyDrop,
    },
    rendy_core::{
        hal, Device,
    },
    rendy_memory::{Heaps, MemoryUsage, Block},
    rendy_resource::{
        ImageInfo, ImageViewInfo, CreationError,
        ViewError, SamplerDesc, Extent,
    },
    rendy_descriptor::DescriptorSetLayoutBinding,
};

use crate::handle::{InstanceStore, EphemerialStore};

use crate::resource::{
    Managed,
    buffer::{ManagedBuffer, ManagedBufferData, BufferMarker},
    image::{ManagedImage, ManagedImageData, ImageMarker},
    image_view::{ManagedImageView, ManagedImageViewData, ImageViewMarker, ImageViewKey},
    shader_module::{ManagedShaderModule, ManagedShaderModuleData,
                    ShaderModuleMarker, ShaderModuleKey, Spirv},
    sampler::{ManagedSampler, ManagedSamplerData, SamplerMarker},
    descriptor_set_layout::{ManagedDescriptorSetLayout, DescriptorSetLayoutKey,
                            DescriptorSetLayoutMarker, ManagedDescriptorSetLayoutData},
    pipeline_layout::{ManagedPipelineLayout, PipelineLayoutKey,
                      PipelineLayoutMarker, ManagedPipelineLayoutData},
    render_pass::{ManagedRenderPass, RenderPassKey, RenderPassMarker,
                  ManagedRenderPassData, SubpassDesc},
    graphics_pipeline::{
        ManagedGraphicsPipeline, GraphicsPipelineKey, GraphicsPipelineMarker,
        ManagedGraphicsPipelineData, GraphicsPipelineDesc, GraphicsShaderSet,
    },
    framebuffer::{
        ManagedFramebuffer, FramebufferKey, FramebufferMarker,
        ManagedFramebufferData,
    },
};

// Image -> ImageView
// Buffer -> BufferView
//
// Sampler -> DescriptorSetLayout
// DescriptorSetLayout -> PipelineLayout
// ShaderModule -> GraphicsPipeline
// PipelineLayout -> GraphicsPipeline
// RenderPass -> GraphicsPipeline
// RenderPass -> Framebuffer
// ImageView -> Framebuffer
//
// DescriptorPool -> DescriptorSet
// DescriptorSetLayout -> DescriptorSet
//
// Sampler -> DescriptorSet
// ImageView -> DescriptorSet
// Buffer -> DescriptorSet
// BufferView -> DescriptorSet

/// ## Caching strategies
/// Different managed resources have different caching strategies.
///
/// ### Instance
/// Only ever explicitly evicted. Used for Image, Buffer.
///
/// The instance strategy doesn't really do caching, simply reference counting.
/// When there are no remaining references to the instance or any derived
/// resources, it will be deallocated immediately.
///
/// ### Ephemerial
/// This strategy is only used for stateless and possibly derived resources.
/// Examples of these would be ImageView, Sampler or RenderPass.
///
/// Once there are no remaining references to a resource, this strategy will
/// evict it according to the caching policy.
pub struct ResourceManager<B>
where
    B: hal::Backend,
{
    //device: Device<B>,
    //heaps: ManuallyDrop<parking_lot::Mutex<Heaps<B>>>,

    buffer_store: InstanceStore<BufferMarker<B>>,
    image_store: InstanceStore<ImageMarker<B>>,
    image_view_store: EphemerialStore<ImageViewMarker<B>>,
    shader_module_store: EphemerialStore<ShaderModuleMarker<B>>,
    sampler_store: EphemerialStore<SamplerMarker<B>>,
    descriptor_set_layout_store: EphemerialStore<DescriptorSetLayoutMarker<B>>,
    pipeline_layout_store: EphemerialStore<PipelineLayoutMarker<B>>,
    render_pass_store: EphemerialStore<RenderPassMarker<B>>,
    graphics_pipeline_store: EphemerialStore<GraphicsPipelineMarker<B>>,
    framebuffer_store: EphemerialStore<FramebufferMarker<B>>,
}

use std::fmt::{Debug, Formatter, Result as FmtResult};
impl<B: hal::Backend> Debug for ResourceManager<B> {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "#ResourceManager")
    }
}

impl<B: hal::Backend> ResourceManager<B> {

    pub fn new(device: &Device<B>) -> Self {
        let id = device.id();
        Self {
            buffer_store: InstanceStore::new(id),
            image_store: InstanceStore::new(id),
            image_view_store: EphemerialStore::new(id),
            shader_module_store: EphemerialStore::new(id),
            sampler_store: EphemerialStore::new(id),
            descriptor_set_layout_store: EphemerialStore::new(id),
            pipeline_layout_store: EphemerialStore::new(id),
            render_pass_store: EphemerialStore::new(id),
            graphics_pipeline_store: EphemerialStore::new(id),
            framebuffer_store: EphemerialStore::new(id),
        }
    }

    // /// Creates a new domain within the FactoryCache.
    // /// The epoch may be advanced either a single or several domains at a time.
    // /// A use-case for this mechanism is async compute, which could run over
    // /// several graphics rendering frames. If a separate domain wasn't used, the
    // /// cached values relevant to the compute could be evicted too quickly.
    // pub fn create_domain(
    //     &mut self,
    // ) -> ManagedDomain
    // {
    //     unimplemented!()
    // }

    /// ## Strategy
    /// This resource uses the instance strategy.
    pub fn create_image(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        heaps: &mut Heaps<B>,
        info: ImageInfo,
        memory_usage: impl MemoryUsage,
    ) -> Result<ManagedImage<B>, CreationError<hal::image::CreationError>>
    {
        let data = ManagedImageData::create(
            device, heaps, info, memory_usage)?;
        let handle = self.image_store.insert(data);
        let managed = self.image_store[handle].clone();
        Ok(managed)
    }

    // /// ## Strategy
    // /// This resource uses the instance strategy.
    // pub fn create_buffer(
    //     &mut self,
    //     domain: ManagedDomain,
    //     info: BufferInfo,
    //     memory_usage: impl MemoryUsage,
    // ) -> Result<ManagedBuffer<B>, CreationError<hal::buffer::CreationError>>
    // {
    //     unimplemented!()
    // }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_image_view(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        image: ManagedImage<B>,
        info: ImageViewInfo,
    ) -> Result<ManagedImageView<B>, CreationError<ViewError>>
    {
        let key = Arc::new(ImageViewKey {
            image: image.handle(),
            info,
        });

        let handle = if let Some(handle) = self.image_view_store.lookup_key(&key) {
            handle
        } else {
            let data = ManagedImageViewData::create(
                device, &image, key.clone())?;
            self.image_view_store.insert(key, data)
        };


        let managed = self.image_view_store[handle].clone();
        Ok(managed)
    }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_shader_module(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        spirv: Spirv,
    ) -> Result<ManagedShaderModule<B>, hal::device::ShaderError>
    {
        let key = Arc::new(ShaderModuleKey {
            spirv,
        });

        let handle = if let Some(handle) = self.shader_module_store.lookup_key(&key) {
            handle
        } else {
            let data = ManagedShaderModuleData::create(device, key.clone())?;
            self.shader_module_store.insert(key, data)
        };

        let managed = self.shader_module_store[handle].clone();
        Ok(managed)
    }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_sampler(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        desc: SamplerDesc,
    ) -> Result<ManagedSampler<B>, hal::device::AllocationError>
    {
        let handle = if let Some(handle) = self.sampler_store.lookup_key(&desc) {
            handle
        } else {
            let data = ManagedSamplerData::create(device, desc.clone())?;
            self.sampler_store.insert(desc, data)
        };

        let managed = self.sampler_store[handle].clone();
        Ok(managed)
    }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_descriptor_set_layout(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        bindings: Vec<DescriptorSetLayoutBinding>,
        immutable_samplers: Vec<ManagedSampler<B>>,
    ) -> Result<ManagedDescriptorSetLayout<B>, hal::device::OutOfMemory>
    {
        let key = Arc::new(DescriptorSetLayoutKey {
            bindings,
            immutable_samplers: immutable_samplers.iter().map(|v| v.handle()).collect(),
        });
        let handle = if let Some(handle) = self.descriptor_set_layout_store.lookup_key(&key) {
            handle
        } else {
            let data = ManagedDescriptorSetLayoutData::create(
                device, key.clone(), immutable_samplers)?;
            self.descriptor_set_layout_store.insert(key, data)
        };

        let managed = self.descriptor_set_layout_store[handle].clone();
        Ok(managed)
    }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_pipeline_layout(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        set_layouts: Vec<ManagedDescriptorSetLayout<B>>,
        push_constants: Vec<(hal::pso::ShaderStageFlags, Range<u32>)>,
    ) -> Result<ManagedPipelineLayout<B>, hal::device::OutOfMemory>
    {
        let key = Arc::new(PipelineLayoutKey {
            set_layouts: set_layouts.iter().map(|v| v.handle()).collect(),
            push_constants,
        });
        let handle = if let Some(handle) = self.pipeline_layout_store.lookup_key(&key) {
            handle
        } else {
            let data = ManagedPipelineLayoutData::create(
                device, key.clone(), set_layouts)?;
            self.pipeline_layout_store.insert(key, data)
        };

        let managed = self.pipeline_layout_store[handle].clone();
        Ok(managed)
    }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_render_pass(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        attachments: Vec<hal::pass::Attachment>,
        subpasses: Vec<SubpassDesc>,
        dependencies: Vec<hal::pass::SubpassDependency>,
    ) -> Result<ManagedRenderPass<B>, hal::device::OutOfMemory>
    {
        let key = Arc::new(RenderPassKey {
            attachments,
            subpasses,
            dependencies,
        });
        let handle = if let Some(handle) = self.render_pass_store.lookup_key(&key) {
            handle
        } else {
            let data = ManagedRenderPassData::create(
                device, key.clone())?;
            self.render_pass_store.insert(key, data)
        };

        let managed = self.render_pass_store[handle].clone();
        Ok(managed)
    }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_graphics_pipeline(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        shaders: GraphicsShaderSet<B>,
        layout: ManagedPipelineLayout<B>,
        pass: ManagedRenderPass<B>,
        subpass: usize,
        desc: GraphicsPipelineDesc,
    ) -> Result<ManagedGraphicsPipeline<B>, hal::pso::CreationError>
    {
        let key = Arc::new(GraphicsPipelineKey {
            shaders,
            layout,
            pass: pass.compat().clone(),
            subpass,
            desc,
        });
        let handle = if let Some(handle) = self.graphics_pipeline_store.lookup_key(&key) {
            handle
        } else {
            let data = ManagedGraphicsPipelineData::create(
                device, key.clone(), &pass, None)?;
            self.graphics_pipeline_store.insert(key, data)
        };

        let managed = self.graphics_pipeline_store[handle].clone();
        Ok(managed)
    }

    /// ## Strategy
    /// This resource uses the ephemerial strategy.
    pub fn create_framebuffer(
        &mut self,
        //domain: ManagedDomain,
        device: &Device<B>,
        pass: &ManagedRenderPass<B>,
        attachments: Vec<ManagedImageView<B>>,
        extent: Extent,
    ) -> Result<ManagedFramebuffer<B>, hal::device::OutOfMemory>
    {
        let key = Arc::new(FramebufferKey {
            pass: pass.compat().clone(),
            attachments,
            extent,
        });
        let handle = if let Some(handle) = self.framebuffer_store.lookup_key(&key) {
            handle
        } else {
            let data = ManagedFramebufferData::create(
                device, key.clone(), &pass)?;
            self.framebuffer_store.insert(key, data)
        };

        let managed = self.framebuffer_store[handle].clone();
        Ok(managed)
    }

    // /// This will immediately try to evict the resource.
    // /// If use tracking is enabled, this will mark all derived resources as
    // /// dead.
    // /// This will fail if there are any usages of the
    // pub fn evict<T>(
    //     &mut self,
    //     to_evict: Managed<T>,
    // ) {
    //     unimplemented!()
    // }

    // /// Advances the epoch of caching strategies.
    // /// Normally performed between each frame.
    // pub fn advance_epoch(
    //     &mut self,
    //     domains: impl IntoIterator<Item = ManagedDomain>,
    // ) {
    //     unimplemented!()
    // }

}
