extern crate ash;
#[macro_use] extern crate failure;
extern crate rendy;
extern crate winit;

use std::{
    ffi::CStr,
    mem::size_of,
    time::{Duration, Instant},
};
use ash::{
    version::DeviceV1_0,
    vk::{
        AccessFlags,
        QueueFlags,
        RenderPass,
        RenderPassCreateInfo,
        AttachmentDescription,
        Pipeline,
        GraphicsPipelineCreateInfo,
        FramebufferCreateInfo,
        Framebuffer,
        AttachmentLoadOp,
        AttachmentStoreOp,
        ImageLayout,
        SubpassDescription,
        PipelineBindPoint,
        AttachmentReference,
        SubpassDependency,
        PipelineShaderStageCreateInfo,
        ShaderStageFlags,
        PipelineVertexInputStateCreateInfo,
        VertexInputBindingDescription,
        VertexInputRate,
        VertexInputAttributeDescription,
        PipelineCache,
        PipelineInputAssemblyStateCreateInfo,
        PrimitiveTopology,
        PipelineViewportStateCreateInfo,
        Viewport,
        Rect2D,
        PipelineRasterizationStateCreateInfo,
        PipelineMultisampleStateCreateInfo,
        PipelineDepthStencilStateCreateInfo,
        CompareOp,
        PipelineColorBlendStateCreateInfo,
        PipelineColorBlendAttachmentState,
        BlendFactor,
        BlendOp,
        ColorComponentFlags,
        PipelineLayoutCreateInfo,
        PipelineStageFlags,
        PipelineLayout,
        ImageViewCreateInfo,
        ImageViewType,
        ImageSubresourceRange,
        ImageAspectFlags,
        ImageView,
        ImageCreateInfo,
        ImageType,
        SampleCountFlags,
        ImageTiling,
        ImageUsageFlags,
        SharingMode,
        Format,
        Extent3D,
        ShaderModuleCreateInfo,
    },
};

use failure::Error;

use rendy::{
    command::{FamilyIndex, NoIndividualReset, Graphics, OwningPool, PrimaryLevel, OneShot},
    factory::{Factory, Config},
    frame::Frames,
    memory::usage::Data,
    mesh::{Mesh, PosColor, AsVertex},
    renderer::{Renderer, RendererBuilder},
    resource::Image,
    shader::compile_to_spirv,
    wsi::Target,
};

use winit::{EventsLoop, WindowBuilder, Window};

struct FramebufferEtc {
    depth: Image,
    depth_view: ImageView,
    color_view: ImageView,
    framebuffer: Framebuffer,
}

struct SimpleRenderer {
    mesh: Mesh,
    target: Target,
    family_index: usize,
    render_pass: RenderPass,
    layout: PipelineLayout,
    pipeline: Pipeline,
    framebuffers: Vec<FramebufferEtc>,
    pool: OwningPool<Graphics>,
}

struct SimpleRendererBuilder {
    window: Window,
    vertices: Vec<PosColor>,
}

impl Renderer<()> for SimpleRenderer {
    type Desc = SimpleRendererBuilder;
    fn run(&mut self, factory: &mut Factory, (): &mut (), frames: &mut Frames) {
        let buffer = self.pool.acquire_buffer(factory.device());

        let buffer = buffer.begin(OneShot, factory.device());

        let buffer = buffer.finish(factory.device());

        // Owned by `self.pool`.
        let (submit, buffer) = buffer.submit_once();
        unsafe {
            buffer.release();
        }

        let ref mut family = factory.families_mut()[self.family_index];
        let ref mut queue = family.queues()[0];
        
        unimplemented!("Submit command buffer to the queue");
    }

    fn dispose(self, factory: &mut Factory, (): &mut ()) {
        drop(self.mesh);
        unsafe {
            for framebuffer in self.framebuffers {
                factory.device().destroy_framebuffer(framebuffer.framebuffer, None);
                factory.device().destroy_image_view(framebuffer.color_view, None);
                factory.device().destroy_image_view(framebuffer.depth_view, None);
                drop(framebuffer.depth);
            }
            factory.device().destroy_pipeline(self.pipeline, None);
            factory.device().destroy_render_pass(self.render_pass, None);
            self.pool.dispose(factory.device());
        }
        factory.destroy_target(self.target);
    }
}

compile_to_spirv!(
    struct VertexShader {
        kind: Vertex,
        lang: GLSL,
        file: "examples/simple.vert",
    }
    
    struct FragmentShader {
        kind: Fragment,
        lang: GLSL,
        file: "examples/simple.frag",
    }
);

impl RendererBuilder<()> for SimpleRendererBuilder {
    type Error = Error;
    type Renderer = SimpleRenderer;

    fn build(self, factory: &mut Factory, (): &mut ()) -> Result<SimpleRenderer, Error> {

        let target = factory.create_target(self.window, 3)?;

        let extent = target.extent();

        let (family_index, _) = factory.families().iter().enumerate().find(|(index, family)| {
            let graphics = family.capability().subset(QueueFlags::GRAPHICS);
            let presentation = factory.target_support(family.index(), &target);
            graphics && presentation
        }).ok_or_else(|| format_err!("Can't find queue capable of graphics and presentation"))?;

        let mesh = Mesh::new()
            .with_vertices(self.vertices)
            .with_prim_type(PrimitiveTopology::TRIANGLE_LIST)
            .build(FamilyIndex(0), factory)
        ?;

        let render_pass = unsafe {
            factory.device().create_render_pass(
                &RenderPassCreateInfo::builder()
                    .attachments(&[
                        AttachmentDescription::builder()
                            .format(target.format())
                            .load_op(AttachmentLoadOp::CLEAR)
                            .store_op(AttachmentStoreOp::STORE)
                            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
                            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
                            .initial_layout(ImageLayout::UNDEFINED)
                            .final_layout(ImageLayout::PRESENT_SRC_KHR)
                            .build(),
                        AttachmentDescription::builder()
                            .format(Format::D32_SFLOAT)
                            .load_op(AttachmentLoadOp::CLEAR)
                            .store_op(AttachmentStoreOp::DONT_CARE)
                            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
                            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
                            .initial_layout(ImageLayout::UNDEFINED)
                            .final_layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                            .build(),
                    ])
                    .subpasses(&[
                        SubpassDescription::builder()
                            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
                            .color_attachments(&[
                                AttachmentReference::builder()
                                    .attachment(0)
                                    .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                                    .build()
                            ])
                            .depth_stencil_attachment(
                                &AttachmentReference::builder()
                                    .attachment(1)
                                    .layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                                    .build()
                            )
                            .build()
                    ])
                    .dependencies(&[
                        SubpassDependency::builder()
                            .src_subpass(!0)
                            .src_stage_mask(PipelineStageFlags::TOP_OF_PIPE)
                            .src_access_mask(AccessFlags::empty())
                            .dst_subpass(0)
                            .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)
                            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                            .build(),
                        SubpassDependency::builder()
                            .src_subpass(0)
                            .src_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)
                            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                            .dst_subpass(!0)
                            .dst_stage_mask(PipelineStageFlags::BOTTOM_OF_PIPE)
                            .dst_access_mask(AccessFlags::empty())
                            .build()
                    ])
                    .build(),
                None,
            )
        }?;

        let layout = unsafe {
            factory.device().create_pipeline_layout(
                &PipelineLayoutCreateInfo::builder()
                    .build(),
                None,
            )
        }?;

        let (vertex, fragment) = unsafe {
            let vertex = factory.device().create_shader_module(
                &ShaderModuleCreateInfo::builder()
                    .code(VertexShader::SPIRV)
                    .build(),
                None,
            )?;

            let fragment = factory.device().create_shader_module(
                &ShaderModuleCreateInfo::builder()
                    .code(FragmentShader::SPIRV)
                    .build(),
                None,
            )?;

            (vertex, fragment)
        };

        let pipeline = unsafe {
            let mut pipelines = factory.device().create_graphics_pipelines(
                PipelineCache::null(),
                &[
                    GraphicsPipelineCreateInfo::builder()
                        .stages(&[
                            PipelineShaderStageCreateInfo::builder()
                                .stage(ShaderStageFlags::VERTEX)
                                .module(vertex)
                                .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
                                .build(),
                            PipelineShaderStageCreateInfo::builder()
                                .stage(ShaderStageFlags::FRAGMENT)
                                .module(fragment)
                                .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
                                .build(),
                        ])
                        .vertex_input_state(
                            &PipelineVertexInputStateCreateInfo::builder()
                                .vertex_binding_descriptions(&[
                                    VertexInputBindingDescription::builder()
                                        .binding(0)
                                        .stride(PosColor::VERTEX.stride)
                                        .input_rate(VertexInputRate::VERTEX)
                                        .build(),
                                ])
                                .vertex_attribute_descriptions(
                                    &PosColor::VERTEX.attributes.iter().enumerate().map(|(location, attribute)|
                                        VertexInputAttributeDescription::builder()
                                            .location(location as u32)
                                            .binding(0)
                                            .format(attribute.format)
                                            .offset(attribute.offset)
                                            .build()
                                    ).collect::<Vec<_>>()
                                )
                                .build()
                        )
                        .input_assembly_state(
                            &PipelineInputAssemblyStateCreateInfo::builder()
                                .topology(PrimitiveTopology::TRIANGLE_LIST)
                                .build()
                        )
                        .viewport_state(
                            &PipelineViewportStateCreateInfo::builder()
                                .viewports(&[
                                    Viewport::builder()
                                        .width(extent.width as f32)
                                        .height(extent.height as f32)
                                        .min_depth(0.0)
                                        .max_depth(1.0)
                                        .build()
                                ])
                                .scissors(&[
                                    Rect2D::builder()
                                        .extent(extent)
                                        .build()
                                ])
                                .build()
                        )
                        .rasterization_state(
                            &PipelineRasterizationStateCreateInfo::builder()
                                .build()
                        )
                        .multisample_state(
                            &PipelineMultisampleStateCreateInfo::builder()
                                .rasterization_samples(SampleCountFlags::TYPE_1)
                                .build()
                        )
                        .depth_stencil_state(
                            &PipelineDepthStencilStateCreateInfo::builder()
                                .depth_test_enable(1)
                                .depth_write_enable(1)
                                .depth_compare_op(CompareOp::LESS)
                                .depth_bounds_test_enable(1)
                                .min_depth_bounds(0.0)
                                .max_depth_bounds(1.0)
                                .build()
                        )
                        .color_blend_state(
                            &PipelineColorBlendStateCreateInfo::builder()
                                .attachments(&[
                                    PipelineColorBlendAttachmentState::builder()
                                        .blend_enable(1)
                                        .src_color_blend_factor(BlendFactor::ONE_MINUS_DST_ALPHA)
                                        .dst_color_blend_factor(BlendFactor::DST_ALPHA)
                                        .color_blend_op(BlendOp::ADD)
                                        .src_alpha_blend_factor(BlendFactor::ONE_MINUS_DST_ALPHA)
                                        .dst_alpha_blend_factor(BlendFactor::ONE)
                                        .alpha_blend_op(BlendOp::ADD)
                                        .color_write_mask(ColorComponentFlags::all())
                                        .build()
                                ])

                        )
                        .layout(layout)
                        .render_pass(render_pass)
                        .base_pipeline_index(-1)
                        .build(),
                ],
                None,
            )
            .map_err(|(_, error)| error)?;

            pipelines.remove(0)
        };
        
        let framebuffers = unsafe {
            factory.target_images(&target)?
                .into_iter()
                .map(|image| {
                    let depth = factory.create_image(
                        ImageCreateInfo::builder()
                            .image_type(ImageType::TYPE_2D)
                            .format(Format::D32_SFLOAT)
                            .extent(Extent3D {
                                width: target.extent().width,
                                height: target.extent().height,
                                depth: 1,
                            })
                            .mip_levels(1)
                            .array_layers(1)
                            .samples(SampleCountFlags::TYPE_1)
                            .tiling(ImageTiling::OPTIMAL)
                            .usage(ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                            .sharing_mode(SharingMode::EXCLUSIVE)
                            .initial_layout(ImageLayout::UNDEFINED)
                            .build(),
                        1,
                        Data,
                    )?;
                    let depth_view = factory.device().create_image_view(
                        &ImageViewCreateInfo::builder()
                            .image(depth.raw())
                            .view_type(ImageViewType::TYPE_2D)
                            .format(Format::D32_SFLOAT)
                            .subresource_range(
                                ImageSubresourceRange::builder()
                                    .aspect_mask(ImageAspectFlags::COLOR)
                                    .level_count(1)
                                    .layer_count(1)
                                    .build()
                            )
                            .build(),
                        None,
                    )?;
                    let color_view = factory.device().create_image_view(
                        &ImageViewCreateInfo::builder()
                            .image(image)
                            .view_type(ImageViewType::TYPE_2D)
                            .format(target.format())
                            .subresource_range(
                                ImageSubresourceRange::builder()
                                    .aspect_mask(ImageAspectFlags::COLOR)
                                    .level_count(1)
                                    .layer_count(1)
                                    .build()
                            )
                            .build(),
                        None,
                    )?;
                    let framebuffer = factory.device().create_framebuffer(
                        &FramebufferCreateInfo::builder()
                            .render_pass(render_pass)
                            .attachments(&[color_view, depth_view])
                            .width(target.extent().width)
                            .height(target.extent().height)
                            .layers(1)
                            .build(),
                        None,
                    )?;

                    Ok(FramebufferEtc {
                        depth,
                        depth_view,
                        color_view,
                        framebuffer,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()
        }?;

        let ref family = factory.families()[family_index];
        let pool = family.create_owning_pool(unsafe {factory.device()}, PrimaryLevel)?.from_flags().unwrap();

        Ok(SimpleRenderer {
            mesh,
            target,
            family_index,
            render_pass,
            layout,
            pipeline,
            framebuffers,
            pool,
        })
    }
}

fn main() -> Result<(), failure::Error> {
    let started = Instant::now();

    env_logger::init();

    let config: Config = Default::default();

    let mut factory: Factory = Factory::new(config)?;

    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rendy example")
        .build(&event_loop)?;

    event_loop.poll_events(|_| ());

    let renderer_builder = SimpleRendererBuilder {
        window,
        vertices: vec![
            PosColor {
                position: [0.0, -0.5, 0.5].into(),
                color: [1.0, 0.0, 0.0, 1.0].into(),
            },
            PosColor {
                position: [-0.5, 0.5, 0.5].into(),
                color: [0.0, 1.0, 0.0, 1.0].into(),
            },
            PosColor {
                position: [0.5, 0.5, 0.5].into(),
                color: [0.0, 0.0, 1.0, 1.0].into(),
            },
        ],
    };

    let renderer = renderer_builder.build(&mut factory, &mut ())?;

    while started.elapsed() < Duration::new(1, 0) {
        event_loop.poll_events(|_| ());
        std::thread::sleep(Duration::new(0, 1_000_000));
    }
    renderer.dispose(&mut factory, &mut ());

    factory.dispose();
    Ok(())
}
