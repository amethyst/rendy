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
    graph::{
        graph::Graph, Cache, GraphBorrowable, GraphConstructCtx, GraphCtx as _,
        GraphicsPipelineBuilder, ImageId, ImageInfo, ImageMode, Node, PassEntityCtx as _, ShaderId,
        ShaderSetKey,
    },
    //graph::{render::*, Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage},
    hal::{self, device::Device, Backend},
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
};

use rendy::mesh::AsVertex;

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        include_str!("../../../rendy/examples/triangle_newgraph/shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "../rendy/examples/triangle_newgraph/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
        include_str!("../../../rendy/examples/triangle_newgraph/shader.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "../rendy/examples/triangle_newgraph/shader.frag").into(),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref SHADERS: rendy_shader::ShaderSourceSet = rendy_shader::ShaderSourceSet::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

pub struct DrawTriangle<B: hal::Backend> {
    vbuf: GraphBorrowable<Escape<Buffer<B>>>,
    shader_id: ShaderId,
}

impl<B: hal::Backend> DrawTriangle<B> {
    pub fn new(factory: &Factory<B>, cache: &Cache<B>) -> Self {
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

        let key = Arc::new(ShaderSetKey {
            source: SHADERS.clone(),
            spec_constants: Default::default(),
        });
        let reflect = SHADERS.reflect().unwrap();
        let shader_id = cache.make_shader_set(factory, key, reflect);

        DrawTriangle {
            vbuf: GraphBorrowable::new(vbuf),
            shader_id,
        }
    }
}

impl<B: hal::Backend> Node<B> for DrawTriangle<B> {
    type Argument = ();
    type Result = ImageId;

    fn construct(
        &mut self,
        factory: &Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        arg: (),
    ) -> Result<ImageId, ()> {
        let image = ctx.create_image(ImageInfo {
            kind: None,
            levels: 1,
            format: hal::format::Format::Bgra8Srgb,
            mode: ImageMode::Clear {
                transient: false,
                clear: hal::command::ClearValue::default(),
            },
        });

        let mut pass = ctx.pass();
        pass.use_color(0, image, false).unwrap();

        let shader_id = self.shader_id;
        let vbuf = self.vbuf.take_borrow();

        pass.commit(move |node, factory, exec_ctx| {
            exec_ctx.bind_graphics_pipeline(
                shader_id,
                GraphicsPipelineBuilder::default().add_blend_desc(hal::pso::ColorMask::all(), None),
            );

            let vbuf_raw = vbuf.raw();
            exec_ctx
                .bind_vertex_buffers(0, std::iter::once((vbuf_raw, hal::buffer::SubRange::WHOLE)));

            let rect = hal::pso::Rect {
                x: 0,
                y: 0,
                w: 500,
                h: 500,
            };
            exec_ctx.set_viewports(
                0,
                std::iter::once(hal::pso::Viewport {
                    rect,
                    depth: 0.0..1.0,
                }),
            );
            exec_ctx.set_scissors(0, std::iter::once(rect));

            exec_ctx.draw(0..3, 0..1);
        });

        Ok(image)
    }
}

//
fn run2<B: Backend>(
    event_loop: EventLoop<()>,
    mut factory: Factory<B>,
    mut families: Families<B>,
    mut surface: Surface<B>,
    window: Window,
) {
    use rendy::command::{CommandPool, Graphics, IndividualReset, MultiShot, NoSimultaneousUse};
    use rendy::core::hal::command::RenderAttachmentInfo;
    use rendy::core::hal::format::{ChannelType, Format};
    use rendy::core::hal::pass::{
        Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc,
    };
    use rendy::core::hal::pso::{AttributeDesc, GraphicsPipelineDesc, VertexBufferDesc};
    use rendy::core::hal::window::PresentMode;
    use rendy::frame::{cirque::CommandCirque, Frames};
    use rendy::resource::Layout;

    let (width, height) = window.inner_size().into();
    let suggested_extent = hal::window::Extent2D { width, height };
    let surface_extent = unsafe {
        surface
            .extent(factory.physical())
            .unwrap_or(suggested_extent)
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

    let (vert_elements, vert_stride, vert_rate) =
        PosColor::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex);

    let rect = rendy_core::hal::pso::Rect {
        x: 0,
        y: 0,
        w: surface_extent.width as i16,
        h: surface_extent.height as i16,
    };

    let clears = vec![hal::command::ClearValue {
        color: hal::command::ClearColor {
            float32: [1.0, 1.0, 1.0, 1.0],
        },
    }];

    let mut frames = rendy_graph::Frames::new(family_id.into());
    let mut graph = Graph::<B>::new(&factory);

    let mut draw_triangle = GraphBorrowable::new(DrawTriangle::new(
        &factory,
        graph.cache(),
    ));

    let mut present = GraphBorrowable::new(rendy::graph::node::Present::new(
        &factory,
        surface,
        surface_extent,
    ));

    let family = families.family_mut(family_id);
    let queue = family.queue_mut(0);

    loop {
        use rendy::graph::{GraphCtx, ImageInfo, ImageMode};

        let image = graph.construct_node(&mut draw_triangle, ());
        graph.construct_node(&mut present, image);

        graph.schedule(&mut frames, queue);

        frames.advance(&factory);
    }
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
