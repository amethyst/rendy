extern crate ash;
#[macro_use] extern crate failure;
#[macro_use] extern crate log;
extern crate rendy;
extern crate winit;

use std::{
    ffi::CStr,
    mem::size_of,
    time::{Duration, Instant},
};
use ash::{
    version::DeviceV1_0,
    vk,
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
    depth_view: vk::ImageView,
    color_view: vk::ImageView,
    framebuffer: vk::Framebuffer,
    acquire: vk::Semaphore,
    release: vk::Semaphore,
    fence: vk::Fence,
    pool: OwningPool<Graphics>,
}

struct SimpleRenderer {
    mesh: Mesh,
    target: Target,
    family_index: FamilyIndex,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    framebuffers: Vec<FramebufferEtc>,
    acquire: vk::Semaphore,
}

struct SimpleRendererBuilder {
    window: Window,
    vertices: Vec<PosColor>,
}

impl Renderer<()> for SimpleRenderer {
    type Desc = SimpleRendererBuilder;

    fn run(&mut self, factory: &mut Factory, (): &mut ()) {
        // trace!("Render frame");

        let next_image = self.target.next_image(self.acquire).unwrap();
        // trace!("Next image acquired");

        let index = next_image.indices()[0];
        let ref mut framebuffer = self.framebuffers[index as usize];
        // trace!("Framebuffer picked");

        factory.wait_for_fence(framebuffer.fence);
        std::mem::swap(&mut self.acquire, &mut framebuffer.acquire);
        // trace!("Got fence signeled");

        unsafe {
            framebuffer.pool.reset(factory.device());
        }
        let buffer = framebuffer.pool.acquire_buffer(factory.device());
        // trace!("Command buffer acquired");

        let buffer = buffer.begin(OneShot, factory.device());
        // trace!("Command buffer begun");

        // TODO: Record commands.

        let buffer = buffer.finish(factory.device());
        // trace!("Command buffer finished");

        let (submit, buffer) = buffer.submit_once();
        unsafe {
            // Owned by `self.pool`.
            buffer.release();
        }

        unsafe {
            let mut queue = factory.queue(self.family_index, 0);
            queue.submit(
                &[
                    vk::SubmitInfo::builder()
                        .wait_semaphores(&[framebuffer.acquire])
                        .signal_semaphores(&[framebuffer.release])
                        .build(),
                ],
                framebuffer.fence,
            );
            // trace!("Command buffer submitted");

            next_image.queue_present(queue.raw(), &[framebuffer.release]).unwrap();
        }
        // trace!("Next image present queued");
    }

    fn dispose(self, factory: &mut Factory, (): &mut ()) {
        factory.queue(self.family_index, 0).wait_idle();
        drop(self.mesh);
        // trace!("Mesh dropped");
        unsafe {
            for framebuffer in self.framebuffers {
                factory.device().destroy_framebuffer(framebuffer.framebuffer, None);
                // trace!("Frambuffer destroyed");
                factory.device().destroy_image_view(framebuffer.color_view, None);
                // trace!("Color view destroyed");
                factory.device().destroy_image_view(framebuffer.depth_view, None);
                // trace!("Depth view destroyed");
                drop(framebuffer.depth);
                // trace!("Depth image destroyed");
                framebuffer.pool.dispose(factory.device());
                // trace!("Pool destroyed");
            }
            factory.device().destroy_pipeline(self.pipeline, None);
            // trace!("Pipeline destroyed");
            factory.device().destroy_render_pass(self.render_pass, None);
            // trace!("Render-pass destroyed");
        }
        factory.destroy_target(self.target);
        // trace!("Target destroyed");
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

        let (index, _) = factory.families().iter().enumerate().find(|(index, family)| {
            let graphics = family.capability().subset(vk::QueueFlags::GRAPHICS);
            let presentation = factory.target_support(family.index(), &target);
            graphics && presentation
        }).ok_or_else(|| format_err!("Can't find queue capable of graphics and presentation"))?;

        let family_index = FamilyIndex(index as u32);

        let mesh = Mesh::new()
            .with_vertices(self.vertices)
            .with_prim_type(vk::PrimitiveTopology::TRIANGLE_LIST)
            .build(FamilyIndex(0), factory)
        ?;

        let render_pass = unsafe {
            factory.device().create_render_pass(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[
                        vk::AttachmentDescription::builder()
                            .format(target.format())
                            .load_op(vk::AttachmentLoadOp::CLEAR)
                            .store_op(vk::AttachmentStoreOp::STORE)
                            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                            .initial_layout(vk::ImageLayout::UNDEFINED)
                            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                            .build(),
                        vk::AttachmentDescription::builder()
                            .format(vk::Format::D32_SFLOAT)
                            .load_op(vk::AttachmentLoadOp::CLEAR)
                            .store_op(vk::AttachmentStoreOp::DONT_CARE)
                            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                            .initial_layout(vk::ImageLayout::UNDEFINED)
                            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                            .build(),
                    ])
                    .subpasses(&[
                        vk::SubpassDescription::builder()
                            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                            .color_attachments(&[
                                vk::AttachmentReference::builder()
                                    .attachment(0)
                                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                                    .build()
                            ])
                            .depth_stencil_attachment(
                                &vk::AttachmentReference::builder()
                                    .attachment(1)
                                    .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                                    .build()
                            )
                            .build()
                    ])
                    .dependencies(&[
                        vk::SubpassDependency::builder()
                            .src_subpass(!0)
                            .src_stage_mask(vk::PipelineStageFlags::TOP_OF_PIPE)
                            .src_access_mask(vk::AccessFlags::empty())
                            .dst_subpass(0)
                            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                            .build(),
                        vk::SubpassDependency::builder()
                            .src_subpass(0)
                            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                            .dst_subpass(!0)
                            .dst_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                            .dst_access_mask(vk::AccessFlags::empty())
                            .build()
                    ])
                    .build(),
                None,
            )
        }?;

        let layout = unsafe {
            factory.device().create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .build(),
                None,
            )
        }?;

        let (vertex, fragment) = unsafe {
            let vertex = factory.device().create_shader_module(
                &vk::ShaderModuleCreateInfo::builder()
                    .code(VertexShader::SPIRV)
                    .build(),
                None,
            )?;

            let fragment = factory.device().create_shader_module(
                &vk::ShaderModuleCreateInfo::builder()
                    .code(FragmentShader::SPIRV)
                    .build(),
                None,
            )?;

            (vertex, fragment)
        };

        let pipeline = unsafe {
            let mut pipelines = factory.device().create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[
                    vk::GraphicsPipelineCreateInfo::builder()
                        .stages(&[
                            vk::PipelineShaderStageCreateInfo::builder()
                                .stage(vk::ShaderStageFlags::VERTEX)
                                .module(vertex)
                                .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
                                .build(),
                            vk::PipelineShaderStageCreateInfo::builder()
                                .stage(vk::ShaderStageFlags::FRAGMENT)
                                .module(fragment)
                                .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
                                .build(),
                        ])
                        .vertex_input_state(
                            &vk::PipelineVertexInputStateCreateInfo::builder()
                                .vertex_binding_descriptions(&[
                                    vk::VertexInputBindingDescription::builder()
                                        .binding(0)
                                        .stride(PosColor::VERTEX.stride)
                                        .input_rate(vk::VertexInputRate::VERTEX)
                                        .build(),
                                ])
                                .vertex_attribute_descriptions(
                                    &PosColor::VERTEX.attributes.iter().enumerate().map(|(location, attribute)|
                                        vk::VertexInputAttributeDescription::builder()
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
                            &vk::PipelineInputAssemblyStateCreateInfo::builder()
                                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                                .build()
                        )
                        .viewport_state(
                            &vk::PipelineViewportStateCreateInfo::builder()
                                .viewports(&[
                                    vk::Viewport::builder()
                                        .width(extent.width as f32)
                                        .height(extent.height as f32)
                                        .min_depth(0.0)
                                        .max_depth(1.0)
                                        .build()
                                ])
                                .scissors(&[
                                    vk::Rect2D::builder()
                                        .extent(extent)
                                        .build()
                                ])
                                .build()
                        )
                        .rasterization_state(
                            &vk::PipelineRasterizationStateCreateInfo::builder()
                                .build()
                        )
                        .multisample_state(
                            &vk::PipelineMultisampleStateCreateInfo::builder()
                                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                                .build()
                        )
                        .depth_stencil_state(
                            &vk::PipelineDepthStencilStateCreateInfo::builder()
                                .depth_test_enable(1)
                                .depth_write_enable(1)
                                .depth_compare_op(vk::CompareOp::LESS)
                                .depth_bounds_test_enable(1)
                                .min_depth_bounds(0.0)
                                .max_depth_bounds(1.0)
                                .build()
                        )
                        .color_blend_state(
                            &vk::PipelineColorBlendStateCreateInfo::builder()
                                .attachments(&[
                                    vk::PipelineColorBlendAttachmentState::builder()
                                        .blend_enable(1)
                                        .src_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
                                        .dst_color_blend_factor(vk::BlendFactor::DST_ALPHA)
                                        .color_blend_op(vk::BlendOp::ADD)
                                        .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
                                        .dst_alpha_blend_factor(vk::BlendFactor::ONE)
                                        .alpha_blend_op(vk::BlendOp::ADD)
                                        .color_write_mask(vk::ColorComponentFlags::all())
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
            target.images()
                .iter()
                .map(|&image| {
                    let depth = factory.create_image(
                        vk::ImageCreateInfo::builder()
                            .image_type(vk::ImageType::TYPE_2D)
                            .format(vk::Format::D32_SFLOAT)
                            .extent(vk::Extent3D {
                                width: target.extent().width,
                                height: target.extent().height,
                                depth: 1,
                            })
                            .mip_levels(1)
                            .array_layers(1)
                            .samples(vk::SampleCountFlags::TYPE_1)
                            .tiling(vk::ImageTiling::OPTIMAL)
                            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                            .sharing_mode(vk::SharingMode::EXCLUSIVE)
                            .initial_layout(vk::ImageLayout::UNDEFINED)
                            .build(),
                        1,
                        Data,
                    )?;
                    let depth_view = factory.device().create_image_view(
                        &vk::ImageViewCreateInfo::builder()
                            .image(depth.raw())
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(vk::Format::D32_SFLOAT)
                            .subresource_range(
                                vk::ImageSubresourceRange::builder()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .level_count(1)
                                    .layer_count(1)
                                    .build()
                            )
                            .build(),
                        None,
                    )?;
                    let color_view = factory.device().create_image_view(
                        &vk::ImageViewCreateInfo::builder()
                            .image(image)
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(target.format())
                            .subresource_range(
                                vk::ImageSubresourceRange::builder()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .level_count(1)
                                    .layer_count(1)
                                    .build()
                            )
                            .build(),
                        None,
                    )?;
                    let framebuffer = factory.device().create_framebuffer(
                        &vk::FramebufferCreateInfo::builder()
                            .render_pass(render_pass)
                            .attachments(&[color_view, depth_view])
                            .width(target.extent().width)
                            .height(target.extent().height)
                            .layers(1)
                            .build(),
                        None,
                    )?;


                    let mut pool = {
                        let ref family = factory.families()[family_index.0 as usize];
                        family.create_owning_pool(unsafe {factory.device()}, PrimaryLevel)?.from_flags().unwrap()
                    };

                    Ok(FramebufferEtc {
                        depth,
                        depth_view,
                        color_view,
                        framebuffer,
                        acquire: factory.create_semaphore(),
                        release: factory.create_semaphore(),
                        fence: factory.create_fence(true),
                        pool,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()
        }?;

        Ok(SimpleRenderer {
            mesh,
            target,
            family_index,
            render_pass,
            layout,
            pipeline,
            framebuffers,
            acquire: factory.create_semaphore(),
        })
    }
}

fn main() -> Result<(), failure::Error> {

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

    let mut renderer = renderer_builder.build(&mut factory, &mut ())?;

    // trace!("Start rendering");
    let mut counter = (0 .. );
    let started = Instant::now();
    counter.by_ref().take_while(|_| started.elapsed() < Duration::new(1, 0)).for_each(|_| {
        event_loop.poll_events(|_| ());
        renderer.run(&mut factory, &mut ());
        // std::thread::sleep(Duration::new(0, 10_000_000));
    });

    info!("Rendered {} frames", counter.start);
    // trace!("Stop rendering");

    renderer.dispose(&mut factory, &mut ());
    // trace!("Render disposed");

    factory.dispose();
    // trace!("Factory disposed");
    Ok(())
}
