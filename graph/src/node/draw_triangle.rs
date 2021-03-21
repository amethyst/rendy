use rendy_core::hal;
use hal::device::Device;

use std::sync::Arc;

use crate::{Node, GraphBorrowable};
use crate::graph::GraphConstructCtx;
use crate::command2::{Cache, ShaderSetKey};
//use crate::builder::{Node, GraphConstructCtx};
use crate::factory::Factory;
use crate::scheduler::{ImageId, interface::{PassEntityCtx, GraphCtx}, resources::{ImageInfo, ImageMode}};
use crate::parameter::{Parameter, ParameterStore};
use crate::resource::{Buffer, BufferInfo, Escape};
use crate::memory::Dynamic;
use crate::exec::GraphicsPipelineBuilder;
use rendy_shader::{ShaderKind, SourceLanguage, SourceShaderInfo, SpirvShader, ShaderId};
use rendy_mesh::{PosColor, AsVertex};

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
            format: hal::format::Format::Bgr8Srgb,
            mode: ImageMode::Clear {
                transient: false,
                clear: hal::command::ClearValue::default(),
            }
        });

        let mut pass = ctx.pass();
        pass.use_color(0, image, false).unwrap();

        let shader_id = self.shader_id;
        let vbuf = self.vbuf.take_borrow();

        pass.commit(move |node, factory, exec_ctx| {
            exec_ctx.bind_graphics_pipeline(
                shader_id,
                GraphicsPipelineBuilder::default(),
            );

            let vbuf_raw = vbuf.raw();
            exec_ctx.bind_vertex_buffers(0, std::iter::once((vbuf_raw, hal::buffer::SubRange::WHOLE)));

            exec_ctx.draw(0..3, 0..1);
        });

        Ok(image)
    }
}
