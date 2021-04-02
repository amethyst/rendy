use std::borrow::Borrow;
use std::sync::Arc;

use rendy::{
    command::{Families, QueueId, RenderPassEncoder, Submission},
    factory::{Config, Factory},
    graph::{
        graph::Graph, Cache, GraphBorrowable, GraphConstructCtx, GraphCtx as _,
        GraphicsPipelineBuilder, ImageId, ImageInfo, ImageMode, Node, PassEntityCtx as _, ShaderId,
        ShaderSetKey, ImageInfoBuilder,
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

        let depth_image = ctx.create_image(
            ImageInfoBuilder::default()
                .with_format(hal::format::Format::D32Sfloat)
                .build()
        );

        let albeido_image = ctx.create_image(
            ImageInfoBuilder::default()
                .with_format(hal::format::Format::Rgba8Unorm)
                .build()
        );

        let target_image = ctx.create_image(
            ImageInfoBuilder::default()
                .with_format(hal::format::Format::Bgra8Srgb)
                .build()
        );

        // G-buffer pass
        {
            let mut pass = ctx.pass();
            pass.use_depth(depth_image, true);
            pass.use_color(0, albeido_image, false);

            pass.commit(move |node, factory, exec_ctx| {
                ()
            });
        }

        // Deferred rendering pass
        {
            let mut pass = ctx.pass();
            pass.use_input(0, depth_image);
            pass.use_input(1, albeido_image);
            pass.use_color(0, target_image, false);

            pass.commit(move |node, factory, exec_ctx| {
                ()
            });
        }


        //let mut pass = ctx.pass();
        //pass.use_color(0, image, false).unwrap();

        //let shader_id = self.shader_id;
        //let vbuf = self.vbuf.take_borrow();

        //pass.commit(move |node, factory, exec_ctx| {
        //    exec_ctx.bind_graphics_pipeline(
        //        shader_id,
        //        GraphicsPipelineBuilder::default().add_blend_desc(hal::pso::ColorMask::all(), None),
        //    );

        //    let vbuf_raw = vbuf.raw();
        //    exec_ctx
        //        .bind_vertex_buffers(0, std::iter::once((vbuf_raw, hal::buffer::SubRange::WHOLE)));

        //    let rect = hal::pso::Rect {
        //        x: 0,
        //        y: 0,
        //        w: 500,
        //        h: 500,
        //    };
        //    exec_ctx.set_viewports(
        //        0,
        //        std::iter::once(hal::pso::Viewport {
        //            rect,
        //            depth: 0.0..1.0,
        //        }),
        //    );
        //    exec_ctx.set_scissors(0, std::iter::once(rect));

        //    exec_ctx.draw(0..3, 0..1);
        //});

        Ok(target_image)
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
    assert!(factory.surface_support(family_id, &surface));

    let (vert_elements, vert_stride, vert_rate) =
        PosColor::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex);

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
