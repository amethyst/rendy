//!
//! A simple sprite example.
//! This examples shows how to render a sprite on a white background.
//!

use rendy_examples::*;

use rendy::{
    command::{Families, QueueId, RenderPassEncoder},
    factory::{Factory, ImageState},
    graph::{render::*, GraphBuilder, GraphContext, NodeBuffer, NodeImage},
    hal::{self, pso::ShaderStageFlags, Device as _},
    memory::Dynamic,
    mesh::PosTex,
    resource::{Buffer, BufferInfo, DescriptorSet, DescriptorSetLayout, Escape, Handle},
    shader::{ShaderSetBuilder, ShaderSet, SpirvShader},
    texture::{image::ImageTextureConfig, Texture},
    util::*,
};

#[cfg(feature = "spirv-reflection")]
use rendy::shader::SpirvReflection;

#[cfg(not(feature = "spirv-reflection"))]
use rendy::mesh::AsVertex;

use std::io::Cursor;


lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SpirvShader::new(
        unsafe {
            let bytes = include_bytes!("vert.spv");
            std::slice::from_raw_parts(bytes.as_ptr() as *const u32, bytes.len() / 4).to_vec()
        },
        ShaderStageFlags::VERTEX,
        "main"
    );

    static ref FRAGMENT: SpirvShader = SpirvShader::new(
        unsafe {
            let bytes = include_bytes!("frag.spv");
            std::slice::from_raw_parts(bytes.as_ptr() as *const u32, bytes.len() / 4).to_vec()
        },
        ShaderStageFlags::FRAGMENT,
        "main",
    );

    static ref SHADERS: ShaderSetBuilder = ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}

#[cfg(feature = "spirv-reflection")]
lazy_static::lazy_static! {
    static ref SHADER_REFLECTION: SpirvReflection = SHADERS.reflect().unwrap();
}

#[derive(Debug, Default)]
struct SpriteGraphicsPipelineDesc;

#[derive(Debug)]
struct SpriteGraphicsPipeline<B: hal::Backend> {
    texture: Texture<B>,
    vbuf: Escape<Buffer<B>>,
    descriptor_set: Escape<DescriptorSet<B>>,
}

impl<B, T> SimpleGraphicsPipelineDesc<B, T> for SpriteGraphicsPipelineDesc
where
    B: hal::Backend,
    T: ?Sized,
{
    type Pipeline = SpriteGraphicsPipeline<B>;

    fn depth_stencil(&self) -> Option<hal::pso::DepthStencilDesc> {
        None
    }

    fn load_shader_set(&self, factory: &mut Factory<B>, _aux: &T) -> ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        #[cfg(feature = "spirv-reflection")]
        return vec![SHADER_REFLECTION
            .attributes_range(..)
            .unwrap()
            .gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)];

        #[cfg(not(feature = "spirv-reflection"))]
        return vec![PosTex::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)];
    }

    fn layout(&self) -> Layout {
        #[cfg(feature = "spirv-reflection")]
        return SHADER_REFLECTION.layout().unwrap();

        #[cfg(not(feature = "spirv-reflection"))]
        return Layout {
            sets: vec![SetLayout {
                bindings: vec![
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 0,
                        ty: hal::pso::DescriptorType::SampledImage,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 1,
                        ty: hal::pso::DescriptorType::Sampler,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                ],
            }],
            push_constants: Vec::new(),
        };
    }

    fn build<'b>(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<SpriteGraphicsPipeline<B>, failure::Error> {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert_eq!(set_layouts.len(), 1);

        let data = get_data!("logo.png")?;

        // This is how we can load an image and create a new texture.
        let texture_builder = rendy::texture::image::load_from_image(
            Cursor::new(&data[..]),
            ImageTextureConfig {
                #[cfg(not(feature = "gl"))]
                generate_mips: true,
                ..Default::default()
            },
        )?;

        let texture = texture_builder
            .build(
                ImageState {
                    queue,
                    stage: hal::pso::PipelineStage::FRAGMENT_SHADER,
                    access: hal::image::Access::SHADER_READ,
                    layout: hal::image::Layout::ShaderReadOnlyOptimal,
                },
                factory,
            )
            .unwrap();

        let descriptor_set = factory
            .create_descriptor_set(set_layouts[0].clone())
            .unwrap();

        unsafe {
            factory.device().write_descriptor_sets(vec![
                hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: vec![hal::pso::Descriptor::Image(
                        texture.view().raw(),
                        hal::image::Layout::ShaderReadOnlyOptimal,
                    )],
                },
                hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 1,
                    array_offset: 0,
                    descriptors: vec![hal::pso::Descriptor::Sampler(texture.sampler().raw())],
                },
            ]);
        }

        #[cfg(feature = "spirv-reflection")]
        let vbuf_size = SHADER_REFLECTION.attributes_range(..).unwrap().stride as u64 * 6;

        #[cfg(not(feature = "spirv-reflection"))]
        let vbuf_size = PosTex::vertex().stride as u64 * 6;

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
                        PosTex {
                            position: [-0.5, 0.33, 0.0].into(),
                            tex_coord: [0.0, 1.0].into(),
                        },
                        PosTex {
                            position: [0.5, 0.33, 0.0].into(),
                            tex_coord: [1.0, 1.0].into(),
                        },
                        PosTex {
                            position: [0.5, -0.33, 0.0].into(),
                            tex_coord: [1.0, 0.0].into(),
                        },
                        PosTex {
                            position: [-0.5, 0.33, 0.0].into(),
                            tex_coord: [0.0, 1.0].into(),
                        },
                        PosTex {
                            position: [0.5, -0.33, 0.0].into(),
                            tex_coord: [1.0, 0.0].into(),
                        },
                        PosTex {
                            position: [-0.5, -0.33, 0.0].into(),
                            tex_coord: [0.0, 0.0].into(),
                        },
                    ],
                )
                .unwrap();
        }

        Ok(SpriteGraphicsPipeline {
            texture,
            vbuf,
            descriptor_set,
        })
    }
}

impl<B, T> SimpleGraphicsPipeline<B, T> for SpriteGraphicsPipeline<B>
where
    B: hal::Backend,
    T: ?Sized,
{
    type Desc = SpriteGraphicsPipelineDesc;

    fn prepare(
        &mut self,
        _factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        _index: usize,
        _aux: &T,
    ) -> PrepareResult {
        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        _index: usize,
        _aux: &T,
    ) {
        unsafe {
            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                std::iter::once(self.descriptor_set.raw()),
                std::iter::empty::<u32>(),
            );
            encoder.bind_vertex_buffers(0, Some((self.vbuf.raw(), 0)));
            encoder.draw(0..6, 0..1);
        }
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &T) {}
}

rendy_wasm32! {
    #[wasm_bindgen(start)]
    pub fn wasm_main() {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        main();
    }
}

fn main() {
    run(|factory, families, surface| {
        let mut graph_builder = GraphBuilder::<Backend, ()>::new();

        graph_builder.add_node(
            SpriteGraphicsPipeline::builder()
                .into_subpass()
                .with_color_surface()
                .into_pass()
                .with_surface(
                    surface,
                    Some(hal::command::ClearValue::Color([1.0, 1.0, 1.0, 1.0].into())),
                ),
        );

        let graph = graph_builder
            .build(factory, families, &())
            .unwrap();

        (graph, (), move |_: &mut Factory<Backend>, _: &mut Families<Backend>, _: &mut ()| true)
    })
}
