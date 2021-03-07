use rendy_core::hal;
use hal::device::Device;

use crate::builder::{Node, GraphConstructCtx};
use crate::factory::Factory;
use crate::scheduler::{ImageId, interface::{PassEntityCtx, GraphCtx}, resources::{ImageInfo, ImageMode}};
use crate::parameter::{Parameter, ParameterStore};
use crate::resource::{Buffer, BufferInfo, Escape};
use crate::memory::Dynamic;
use rendy_shader::{ShaderKind, SourceLanguage, SourceShaderInfo, SpirvShader};
use rendy_mesh::{PosColor, AsVertex};

lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SourceShaderInfo::new(
        "", //include_str!("shader.vert"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangle/shader.vert").into(),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref FRAGMENT: SpirvShader = SourceShaderInfo::new(
        "", //include_str!("shader.frag"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/triangle/shader.frag").into(),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    ).precompile().unwrap();

    static ref SHADERS: rendy_shader::ShaderSetBuilder = rendy_shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

pub struct DrawTriangle<B: hal::Backend> {
    vbuf: Escape<Buffer<B>>,
}

impl<B: hal::Backend> DrawTriangle<B> {

    pub fn new(factory: &Factory<B>) -> Self {
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

        let mut shader_set = SHADERS
            .build(&factory, Default::default())
            .unwrap();

        #[cfg(not(feature = "spirv-reflection"))]
        let vbuf_size = PosColor::vertex().stride as u64 * 3;

        #[cfg(not(feature = "spirv-reflection"))]
        let (vert_elements, vert_stride, vert_rate) =
            PosColor::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex);

        let rect = rendy_core::hal::pso::Rect {
            x: 0,
            y: 0,
            w: 100 as i16,
            h: 100 as i16,
        };

        let graphics_pipeline = unsafe {
            factory.device().create_graphics_pipeline(
                &hal::pso::GraphicsPipelineDesc {
                    label: None,

                    primitive_assembler: hal::pso::PrimitiveAssemblerDesc::Vertex {
                        buffers: &[
                            hal::pso::VertexBufferDesc {
                                binding: 0,
                                stride: vert_stride,
                                rate: vert_rate,
                            },
                        ],
                        attributes: &vert_elements.iter().enumerate().map(|(idx, elem)| {
                            hal::pso::AttributeDesc {
                                location: idx as u32,
                                binding: 0,
                                element: *elem,
                            }
                        }).collect::<Vec<_>>(),
                        input_assembler: hal::pso::InputAssemblerDesc {
                            primitive: hal::pso::Primitive::TriangleList,
                            with_adjacency: false,
                            restart_index: None,
                        },
                        vertex: shader_set.raw_vertex().unwrap().unwrap(),
                        tessellation: None,
                        geometry: shader_set.raw_geometry().unwrap(),
                    },
                    rasterizer: hal::pso::Rasterizer::FILL,
                    fragment: shader_set.raw_fragment().unwrap(),

                    blender: hal::pso::BlendDesc {
                        logic_op: None,
                        targets: vec![
                            hal::pso::ColorBlendDesc {
                                mask: hal::pso::ColorMask::ALL,
                                blend: None,
                            },
                        ],
                    },
                    depth_stencil: hal::pso::DepthStencilDesc {
                        depth: None,
                        depth_bounds: false,
                        stencil: None,
                    },
                    multisampling: None,
                    baked_states: hal::pso::BakedStates {
                        viewport: Some(hal::pso::Viewport {
                            rect,
                            depth: (0.0.into())..(1.0.into()),
                        }),
                        scissor: Some(rect),
                        blend_color: None,
                        depth_bounds: None,
                    },
                    layout: &pipeline_layout,
                    subpass: hal::pass::Subpass {
                        index: 0,
                        main_pass: &render_pass,
                    },
                    flags: hal::pso::PipelineCreationFlags::empty(),
                    parent: hal::pso::BasePipeline::None,
                },
                None,
            ).unwrap()
        };

        DrawTriangle {
            vbuf,
        }
    }

}

impl<B: hal::Backend> Node<B> for DrawTriangle<B> {
    type Result = Parameter<ImageId>;

    fn construct(
        &mut self,
        factory: &mut Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        store: &ParameterStore,
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

        pass.commit(|node, factory, exec_ctx, encoder| {

        });

        Ok(image)
    }
}
