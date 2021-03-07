//!
//! The mighty triangle example.
//! This examples shows colord triangle on white background.
//! Nothing fancy. Just prove that `rendy` works.
//!

use std::borrow::Borrow;
use std::sync::Arc;

use rendy::{
    command::{Families, QueueId, RenderPassEncoder, Submission},
    factory::{Config, Factory},
    //graph::{render::*, Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage},
    hal::{self, Backend, device::Device},
    init::winit::{
        dpi::PhysicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::{Window, WindowBuilder},
    },
    init::AnyWindowedRendy,
    memory::Dynamic,
    mesh::PosColor,
    resource::{Buffer, BufferInfo, DescriptorSetLayout, Escape, Handle},
    shader::{ShaderKind, SourceLanguage, SourceShaderInfo, SpirvShader},
    wsi::Surface,
    graph::graph::Graph,
};

#[cfg(feature = "spirv-reflection")]
use rendy::shader::SpirvReflection;

#[cfg(not(feature = "spirv-reflection"))]
use rendy::mesh::AsVertex;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangle/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
        include_str!("shader.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangle/shader.frag").into(),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

#[cfg(feature = "spirv-reflection")]
lazy_static::lazy_static! {
    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}

fn run2<B: Backend>(
    event_loop: EventLoop<()>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    mut surface: Surface<B>,
    window: Window,
) {

    use rendy::resource::Layout;
    use rendy::core::hal::format::{Format, ChannelType};
    use rendy::core::hal::pass::{Attachment, AttachmentOps, AttachmentLoadOp, AttachmentStoreOp, SubpassDesc};
    use rendy::core::hal::window::PresentMode;
    use rendy::core::hal::pso::{GraphicsPipelineDesc, VertexBufferDesc, AttributeDesc};
    use rendy::core::hal::command::RenderAttachmentInfo;
    use rendy::command::{CommandPool, Graphics, IndividualReset, MultiShot, NoSimultaneousUse};
    use rendy::frame::{Frames, cirque::CommandCirque};

    let (width, height) = window.inner_size().into();
    let suggested_extent = hal::window::Extent2D { width, height };
    let surface_extent = unsafe {
        surface.extent(factory.physical()).unwrap_or(suggested_extent)
    };
    println!("surface extent: {:?}", surface_extent);

    println!("families: {:?}", families);

    let family_id = families.with_capability::<Graphics>().unwrap();
    let queue_idx: usize = 0;
    println!("selected family: {:?}", family_id);

    assert!(factory.surface_support(family_id, &surface));

    let caps = factory.get_surface_capabilities(&surface);
    let formats = factory.get_surface_formats(&surface);

    let present_mode = match () {
        _ if caps.present_modes.contains(PresentMode::FIFO) => PresentMode::FIFO,
        _ if caps.present_modes.contains(PresentMode::MAILBOX) => PresentMode::MAILBOX,
        _ if caps.present_modes.contains(PresentMode::RELAXED) => PresentMode::RELAXED,
        _ if caps.present_modes.contains(PresentMode::IMMEDIATE) => PresentMode::IMMEDIATE,
        _ => panic!("No known present modes found"),
    };

    let img_count_caps = caps.image_count.clone();
    let image_count = 3.min(*img_count_caps.end()).max(*img_count_caps.start());

    let format = formats.map_or(Format::Rgba8Srgb, |formats| {
        formats
            .iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .map(|format| *format)
            .unwrap_or(formats[0])
    });

    //let swapchain_config = rendy_core::hal::window::SwapchainConfig::from_caps(&caps, format, surface_extent);
    //let fat = swapchain_config.framebuffer_attachment();

    //unsafe {
    //    surface.configure_swapchain(
    //        factory.device(),
    //        swapchain_config,
    //    ).unwrap();
    //}

    let render_pass = unsafe {
        factory.device().create_render_pass(
            [
                Attachment {
                    format: Some(format),
                    samples: 1,
                    ops: AttachmentOps {
                        load: AttachmentLoadOp::Clear,
                        store: AttachmentStoreOp::Store,
                    },
                    stencil_ops: AttachmentOps {
                        load: AttachmentLoadOp::DontCare,
                        store: AttachmentStoreOp::DontCare,
                    },
                    layouts: Layout::Undefined..Layout::Present,
                },
            ].iter().cloned(),
            [
                SubpassDesc {
                    colors: &[
                        (0, Layout::ColorAttachmentOptimal),
                    ],
                    depth_stencil: None,
                    inputs: &[],
                    resolves: &[],
                    preserves: &[],
                },
            ].iter().cloned(),
            None.iter().cloned(),
            ).unwrap()
    };

    //let fb = unsafe {
    //    factory
    //    .device()
    //    .create_framebuffer(
    //        &render_pass,
    //        [fat].iter().cloned(),
    //        rendy::core::hal::image::Extent {
    //            width: surface_extent.width,
    //            height: surface_extent.height,
    //            depth: 1,
    //        },
    //    )
    //    .unwrap()
    //};

    let pipeline_layout = unsafe {
        factory.device().create_pipeline_layout(
            None.iter().cloned(),
            None.iter().cloned(),
            ).unwrap()
    };

    let mut shader_set = SHADERS
        .build(&factory, Default::default())
        .unwrap();

    #[cfg(feature = "spirv-reflection")]
    let vbuf_size = SHADER_REFLECTION.attributes_range(..).unwrap().stride as u64 * 3;

    #[cfg(not(feature = "spirv-reflection"))]
    let vbuf_size = PosColor::vertex().stride as u64 * 3;

    let mut vbuf = factory
        .create_buffer(
            BufferInfo {
                size: vbuf_size,
                usage: hal::buffer::Usage::VERTEX,
            },
            Dynamic,
            )
        .unwrap();

    unsafe {
        // Fresh buffer.
        factory
            .upload_visible_buffer(
                &mut vbuf,
                0,
                &[
                PosColor {
                    position: [0.0, -0.5, 0.0].into(),
                    color: [1.0, 0.0, 0.0, 1.0].into(),
                },
                PosColor {
                    position: [0.5, 0.5, 0.0].into(),
                    color: [0.0, 1.0, 0.0, 1.0].into(),
                },
                PosColor {
                    position: [-0.5, 0.5, 0.0].into(),
                    color: [0.0, 0.0, 1.0, 1.0].into(),
                },
                ],
                )
            .unwrap();
    }

    #[cfg(feature = "spirv-reflection")]
    let (vert_elements, vert_stride, vert_rate) = SHADER_REFLECTION
        .attributes_range(..)
        .unwrap()
        .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex);

    #[cfg(not(feature = "spirv-reflection"))]
    let (vert_elements, vert_stride, vert_rate) = 
        PosColor::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex);

    let rect = rendy_core::hal::pso::Rect {
        x: 0,
        y: 0,
        w: surface_extent.width as i16,
        h: surface_extent.height as i16,
    };

    let graphics_pipeline = unsafe {
        factory.device().create_graphics_pipeline(
            &GraphicsPipelineDesc {
                label: None,

                primitive_assembler: rendy::core::hal::pso::PrimitiveAssemblerDesc::Vertex {
                    buffers: &[
                        VertexBufferDesc { 
                            binding: 0, 
                            stride: vert_stride,
                            rate: vert_rate,
                        },
                    ],
                    attributes: &vert_elements.iter().enumerate().map(|(idx, elem)| {
                        AttributeDesc {
                            location: idx as u32,
                            binding: 0,
                            element: *elem,
                        }
                    }).collect::<Vec<_>>(),
                    input_assembler: rendy::core::hal::pso::InputAssemblerDesc {
                        primitive: rendy::core::hal::pso::Primitive::TriangleList,
                        with_adjacency: false,
                        restart_index: None,
                    },
                    vertex: shader_set.raw_vertex().unwrap().unwrap(),
                    tessellation: None,
                    geometry: shader_set.raw_geometry().unwrap(),
                },
                rasterizer: rendy::core::hal::pso::Rasterizer::FILL,
                fragment: shader_set.raw_fragment().unwrap(),

                blender: rendy::core::hal::pso::BlendDesc {
                    logic_op: None,
                    targets: vec![
                        rendy::core::hal::pso::ColorBlendDesc {
                            mask: rendy::core::hal::pso::ColorMask::ALL,
                            blend: None,
                        },
                    ],
                },
                depth_stencil: rendy::core::hal::pso::DepthStencilDesc {
                    depth: None,
                    depth_bounds: false,
                    stencil: None,
                },
                multisampling: None,
                baked_states: rendy_core::hal::pso::BakedStates {
                    viewport: Some(rendy_core::hal::pso::Viewport {
                        rect,
                        depth: (0.0.into())..(1.0.into()),
                    }),
                    scissor: Some(rect),
                    blend_color: None,
                    depth_bounds: None,
                },
                layout: &pipeline_layout,
                subpass: rendy_core::hal::pass::Subpass {
                    index: 0,
                    main_pass: &render_pass,
                },
                flags: rendy::core::hal::pso::PipelineCreationFlags::empty(),
                parent: rendy::core::hal::pso::BasePipeline::None,
            },
            None,
            ).unwrap()
    };

    let clears = vec![
        hal::command::ClearValue {
            color: hal::command::ClearColor {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        },
    ];

    let mut command_pool: CommandPool<_, Graphics, IndividualReset> = factory
        .create_command_pool(families.family(family_id))
        .unwrap()
        .with_capability()
        .unwrap();

    //let mut command_cirque: CommandCirque<_, Graphics> = CommandCirque::new();

    //let mut frames = Frames::new();

    let mut free_acquire = factory.create_semaphore().unwrap();
    let mut release: Vec<_> = (0..image_count).map(|_| factory.create_semaphore().unwrap()).collect();



    {
        use rendy::graph::{GraphCtx, ImageInfo, ImageMode};
        use rendy::graph::GraphBorrowable;

        let mut graph = Graph::<B>::new(&factory);

        let image = graph.create_image(ImageInfo {
            kind: None,
            levels: 1,
            format: Format::Bgr8Unorm,
            mode: ImageMode::DontCare { transient: false },
        });

        let mut com_pool = unsafe {
            factory.device().create_command_pool(
                family_id.into(),
                hal::pool::CommandPoolCreateFlags::empty(),
            ).unwrap()
        };

        let mut present = GraphBorrowable::new(
            rendy::graph::node::Present::new(&factory, surface, surface_extent));
        graph.construct_node(&mut present, image);

        let family = families.family_mut(family_id);
        let queue = family.queue_mut(0);
        graph.schedule(&mut com_pool, queue);
    }




    //let started = std::time::Instant::now();

    //let mut frame = 0u64;
    //let mut elapsed = started.elapsed();

    //let mut release_idx: usize = 0;

    //event_loop.run(move |event, _, control_flow| {
    //    *control_flow = ControlFlow::Poll;
    //    match event {
    //        Event::WindowEvent { event, .. } => match event {
    //            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
    //            _ => {}
    //        },
    //        Event::MainEventsCleared => {
    //            factory.maintain(&mut families);
    //            frame += 1;
    //            //if let Some(ref mut graph) = graph {
    //                //graph.run(&mut factory, &mut families, &());
    //            //}

    //            release_idx += 1;
    //            if release_idx >= image_count as _ {
    //                release_idx = 0;
    //            }

    //            let family = families.family_mut(family_id);

    //            let sc_image = match unsafe { surface.acquire_image(!0) } {
    //                Ok((image, _suboptimal)) => {
    //                    log::trace!("Presentable image acquired: {:#?}", image);
    //                    image
    //                },
    //                Err(err) => {
    //                    log::debug!("Swapchain acquisition error: {:#?}", err);
    //                    return;
    //                },
    //            };
    //
    //            let submit = command_cirque.encode(&frames, &mut command_pool, |mut cbuf| {
    //                let index = cbuf.index();

    //                cbuf.or_init(|cbuf| {
    //                    let mut cbuf = cbuf.begin(MultiShot(NoSimultaneousUse), ());
    //                    let mut encoder = cbuf.encoder();

    //                    let area = rendy_core::hal::pso::Rect {
    //                        x: 0,
    //                        y: 0,
    //                        w: surface_extent.width as _,
    //                        h: surface_extent.height as _,
    //                    };

    //                    let mut pass_encoder = encoder.begin_render_pass_inline(
    //                        &render_pass,
    //                        &fb,
    //                        area,
    //                        std::iter::once(RenderAttachmentInfo {
    //                            image_view: sc_image.borrow(),
    //                            clear_value: hal::command::ClearValue {
    //                                color: hal::command::ClearColor {
    //                                    float32: [1.0, 1.0, 1.0, 1.0],
    //                                },
    //                            },
    //                        }),
    //                    );

    //                    pass_encoder.bind_graphics_pipeline(&graphics_pipeline);
    //                    unsafe {
    //                        pass_encoder.bind_vertex_buffers(0, Some((vbuf.raw(), 0)));
    //                        pass_encoder.draw(0..3, 0..1);
    //                    }

    //                    drop(pass_encoder);

    //                    cbuf.finish()
    //                })
    //            });

    //            {
    //                let queue = family.queue_mut(0);
    //                unsafe {
    //                    queue.submit(
    //                        Some(
    //                            Submission::new()
    //                                .submits(Some(submit))
    //                                .signal(std::iter::once(&release[release_idx]))
    //                        ),
    //                        None,
    //                    );
    //                }

    //                log::trace!("Present");
    //                if let Err(err) = unsafe { surface.present(queue.raw(), sc_image, Some(&mut release[release_idx])) } {
    //                    log::debug!("Swapchain presentation error: {:#?}", err);
    //                }

    //            }

    //            elapsed = started.elapsed();
    //            if elapsed >= std::time::Duration::new(5, 0) {
    //                *control_flow = ControlFlow::Exit
    //            }
    //        }
    //        _ => {}
    //    }

    //    if *control_flow == ControlFlow::Exit {
    //        let elapsed_ns = elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64;

    //        log::info!(
    //            "Elapsed: {:?}. Frames: {}. FPS: {}",
    //            elapsed,
    //            frame,
    //            frame * 1_000_000_000 / elapsed_ns
    //        );

    //        shader_set.dispose(&factory);
    //    }
    //});




}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("triangle", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(960, 640))
        .with_title("Rendy example");

    let rendy = AnyWindowedRendy::init_auto(&config, window, &event_loop).unwrap();
    rendy::with_any_windowed_rendy!((rendy)(mut factory, mut families, surface, window) => {
        run2(event_loop, factory, families, surface, window);
    });
}
