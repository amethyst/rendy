use std::borrow::Cow;
use std::ops::Range;
use std::hash::{Hash, Hasher};
use std::fmt::Debug;

use rendy_core::hal::{self, pso};

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveAssemblerKind {
    Vertex,
    Mesh,
}

//#[derive(Debug, Clone, PartialEq)]
//pub enum MaybeInfer<T> {
//    None,
//    Infer,
//    Some(T),
//}
//
//#[derive(Debug, Clone, PartialEq)]
//pub struct BakedStates {
//    pub viewport: MaybeInfer<pso::Viewport>,
//    pub scissor: MaybeInfer<pso::Rect>,
//    pub blend_color: Option<pso::ColorValue>,
//    pub depth_bounds: Option<Range<f32>>,
//}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsPipelineBuilder {
    pub label: Option<Cow<'static, str>>,

    // Primitive assembler
    pub primitive_assembler_kind: PrimitiveAssemblerKind,
    pub input_assembler: pso::InputAssemblerDesc,
    pub tessellation: bool,
    pub geometry: bool,
    pub task: bool,

    pub rasterizer: pso::Rasterizer,

    pub fragment: bool,

    pub blender: pso::BlendDesc,

    pub depth_stencil: pso::DepthStencilDesc,

    pub multisampling: Option<pso::Multisampling>,

    // TODO Add baked states if we need em
    //pub baked_states: BakedStates,
}

impl Default for GraphicsPipelineBuilder {
    fn default() -> Self {
        Self {
            label: None,

            primitive_assembler_kind: PrimitiveAssemblerKind::Vertex,
            input_assembler: pso::InputAssemblerDesc {
                primitive: pso::Primitive::TriangleList,
                with_adjacency: false,
                restart_index: None,
            },
            tessellation: false,
            geometry: false,
            task: false,

            rasterizer: pso::Rasterizer::FILL,

            fragment: true,

            blender: pso::BlendDesc {
                logic_op: None,
                targets: Vec::new(),
            },

            depth_stencil: pso::DepthStencilDesc {
                depth: Some(pso::DepthTest {
                    fun: pso::Comparison::Greater,
                    write: true,
                }),
                depth_bounds: true,
                stencil: None,
            },

            multisampling: None,

            //baked_states: BakedStates {
            //    viewport: MaybeInfer::Infer,
            //    scissor: MaybeInfer::Infer,
            //    blend_color: None,
            //    depth_bounds: None,
            //},
        }
    }
}

impl GraphicsPipelineBuilder {

    /// Set the primitive assembler to vertex mode.
    ///
    /// This is the default. It will consume
    ///
    /// ## Shaders
    /// This requires a vertex shader to be present.
    ///
    /// Tessellation shaders (hull, domain) and/or a geometry shader may
    /// optionally also be present.
    pub fn with_vertex_assembler(mut self) -> Self {
        self.primitive_assembler_kind = PrimitiveAssemblerKind::Vertex;
        self
    }

    pub fn with_input_assembler(mut self, assembler: pso::InputAssemblerDesc) -> Self {
        self.input_assembler = assembler;
        self
    }

    pub fn with_assembler_primitive(mut self, primitive: pso::Primitive) -> Self {
        self.input_assembler.primitive = primitive;
        self
    }

    pub fn with_assembler_adjacency(mut self, flag: bool) -> Self {
        self.input_assembler.with_adjacency = flag;
        self
    }

    /// Controls whether a special vertex index value is treated as restarting
    /// the assembly of primitives. This enable only applies to indexed draws.
    /// (draw_indexed and draw_indexed_indirect)
    ///
    /// The special index value depends on the index type:
    /// * For U16, the special value is 0xFFFF
    /// * For U32, the special value is 0xFFFFFFFF
    ///
    /// Primitive restart is not allowed for list primitive topologies.
    pub fn with_assembler_restart_index(mut self, restart_index: hal::IndexType) -> Self {
        self.input_assembler.restart_index = Some(restart_index);
        self
    }

    /// For the vertex assembler mode, tessellation may be enabled.
    ///
    /// If tessellation is enabled, hull and domain shaders are required to be
    /// present in the input shader set.
    pub fn with_assembler_tessellation(mut self, enable: bool) -> Self {
        self.tessellation = enable;
        self
    }

    /// Sets the primitive assembler to mesh shader mode.
    ///
    /// ## Shaders
    /// This requires a mesh shader to be present.
    ///
    /// A task shader may also optionally be present.
    pub fn with_mesh_assembler(mut self) -> Self {
        self.primitive_assembler_kind = PrimitiveAssemblerKind::Mesh;
        self
    }

    pub fn add_blend_desc(mut self, mask: hal::pso::ColorMask, blend: Option<hal::pso::BlendState>) -> Self {
        self.blender.targets.push(hal::pso::ColorBlendDesc {
            mask,
            blend,
        });
        self
    }

}

impl GraphicsPipelineBuilder {

    //pub fn build(&self) ->

}
