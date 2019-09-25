use rendy::{
    command::{Families, QueueId, RenderPassEncoder},
    factory::{Config, Factory},
    graph::{
        render::{
            PrepareResult, RenderGroupBuilder, SimpleGraphicsPipeline, SimpleGraphicsPipelineDesc,
        },
        Graph, GraphBuilder, GraphContext, NodeBuffer, NodeImage,
    },
    hal::{
        self,
        command::{ClearColor, ClearValue},
        pso::DepthStencilDesc,
        Backend as HalBackend,
    },
    memory::Dynamic,
    mesh::{AsVertex, Position},
    resource::{Buffer, BufferInfo, DescriptorSetLayout, Escape, Handle},
    shader::{
        ShaderKind, ShaderSet, ShaderSetBuilder, SourceLanguage, SourceShaderInfo, SpecConstantSet,
    },
    wsi::winit::window::Window,
};

pub type Backend = rendy::metal::Backend;

pub struct Graphics {
    factory: Factory<Backend>,
    families: Families<Backend>,
    user_data: CustomUserData,
    graph: Option<Graph<Backend, CustomUserData>>,
}

impl Drop for Graphics {
    fn drop(&mut self) {
        if let Some(graph) = self.graph.take() {
            graph.dispose(&mut self.factory, &self.user_data);
        }
    }
}

impl Graphics {
    pub fn new(window: &Window) -> Self {
        let config: Config = Config::default();

        let (mut factory, mut families): (Factory<Backend>, _) =
            rendy::factory::init(config).expect("Unable to initialize rendy factory");

        let mut graph_builder = GraphBuilder::<Backend, CustomUserData>::new();
        let user_data = ();

        let surface = factory.create_surface(window);

        graph_builder.add_node(
            RenderPipeline::builder()
                .into_subpass()
                .with_color_surface()
                .into_pass()
                .with_surface(
                    surface,
                    Some(ClearValue {
                        color: ClearColor {
                            float32: [0.392, 0.584, 0.929, 1.0],
                        },
                    }),
                ),
        );

        let graph = Some(
            graph_builder
                .build(&mut factory, &mut families, &user_data)
                .unwrap(),
        );

        Graphics {
            factory,
            families,
            user_data,
            graph,
        }
    }

    pub fn maintain(&mut self) {
        self.factory.maintain(&mut self.families);

        if let Some(graph) = &mut self.graph {
            graph.run(&mut self.factory, &mut self.families, &self.user_data);
        }
    }
}

type CustomUserData = ();

#[derive(Debug, Default)]
struct RenderPipelineDesc;

impl SimpleGraphicsPipelineDesc<Backend, CustomUserData> for RenderPipelineDesc {
    type Pipeline = RenderPipeline;

    fn load_shader_set(
        &self,
        factory: &mut Factory<Backend>,
        _user_data: &CustomUserData,
    ) -> ShaderSet<Backend> {
        let vertex_shader = SourceShaderInfo::new(
            include_str!("shader.vert"),
            "shader.vert",
            ShaderKind::Vertex,
            SourceLanguage::GLSL,
            "main",
        )
        .precompile()
        .expect("Unable to compile vertex shader");

        let fragment_shader = SourceShaderInfo::new(
            include_str!("shader.frag"),
            "shader.frag",
            ShaderKind::Fragment,
            SourceLanguage::GLSL,
            "main",
        )
        .precompile()
        .expect("Unable to compile fragment shader");

        let builder = ShaderSetBuilder::default()
            .with_vertex(&vertex_shader)
            .unwrap()
            .with_fragment(&fragment_shader)
            .unwrap();

        builder
            .build(factory, SpecConstantSet::default())
            .expect("Unable to build source shaders")
    }

    fn vertices(
        &self,
    ) -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
        hal::pso::VertexInputRate,
    )> {
        return vec![Position::vertex().gfx_vertex_input_desc(hal::pso::VertexInputRate::Vertex)];
    }

    fn depth_stencil(&self) -> Option<DepthStencilDesc> {
        None
    }

    fn build(
        self,
        _ctx: &GraphContext<Backend>,
        factory: &mut Factory<Backend>,
        _queue: QueueId,
        _user_data: &CustomUserData,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
        _set_layouts: &[Handle<DescriptorSetLayout<Backend>>],
    ) -> Result<Self::Pipeline, hal::pso::CreationError> {
        let vertex_buffer_size = Position::vertex().stride as u64 * 3;

        println!("vertex_buffer_size is {}", vertex_buffer_size);

        let mut vertex_buffer = factory
            .create_buffer(
                BufferInfo {
                    size: vertex_buffer_size,
                    usage: hal::buffer::Usage::VERTEX,
                },
                Dynamic,
            )
            .unwrap();

        let offset = 0;
        let buffer_content = [
            Position([0.0, -0.5, 1.0]),
            Position([0.5, 0.5, 1.0]),
            Position([-0.5, 0.5, 1.0]),
        ];

        println!("Uploading vertex buffer content");

        unsafe {
            factory
                .upload_visible_buffer(&mut vertex_buffer, offset, &buffer_content)
                .unwrap();
        }

        Ok(RenderPipeline { vertex_buffer })
    }
}

#[derive(Debug)]
struct RenderPipeline {
    vertex_buffer: Escape<Buffer<Backend>>,
}

impl SimpleGraphicsPipeline<Backend, CustomUserData> for RenderPipeline {
    type Desc = RenderPipelineDesc;

    fn prepare(
        &mut self,
        _factory: &Factory<Backend>,
        _queue: QueueId,
        _sets: &[Handle<DescriptorSetLayout<Backend>>],
        index: usize,
        _aux: &CustomUserData,
    ) -> PrepareResult {
        println!("Preparing to draw, index = {}", index);
        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        _layout: &<Backend as HalBackend>::PipelineLayout,
        mut encoder: RenderPassEncoder<Backend>,
        index: usize,
        _user_data: &CustomUserData,
    ) {
        println!("Drawing, index = {}", index);
        unsafe {
            encoder.bind_vertex_buffers(0, Some((self.vertex_buffer.raw(), 0)));
            encoder.draw(0..3, 0..1);
        }
    }

    fn dispose(self, _factory: &mut Factory<Backend>, _user_data: &CustomUserData) {
        println!("dispose() was called");
    }
}
