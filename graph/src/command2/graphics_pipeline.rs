use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::fmt::Debug;

use rendy_core::hal::{self, pso};

pub trait HashableGraphicsPipelineTypes: Debug {
    type Program: Debug + Clone + Eq + PartialEq + Hash;
    type Subpass: Debug + Clone + Eq + PartialEq + Hash;
}

#[derive(Debug, Eq, PartialEq)]
pub enum HashablePrimitiveAssemblerDescr {
    Vertex {
        input_assembler: pso::InputAssemblerDesc,
        tessellation: bool,
        geometry: bool,
    },
    Mesh {
        task: bool,
    },
}
impl Hash for HashablePrimitiveAssemblerDescr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            HashablePrimitiveAssemblerDescr::Vertex {
                input_assembler,
                tessellation,
                geometry,
            } => {
                state.write_u8(0);
                {
                    // input assembler desc
                    input_assembler.primitive.hash(state);
                    input_assembler.with_adjacency.hash(state);
                    input_assembler.restart_index.hash(state);
                }
                tessellation.hash(state);
                geometry.hash(state);
            },
            HashablePrimitiveAssemblerDescr::Mesh { task } => {
                state.write_u8(1);
                task.hash(state);
            },
        }
    }
}

/// Generic hashable description of a pipeline.
///
/// Target specific types are pluggable:
/// * Shader program and pipeline layout within the `program` field
/// * Render pass/subpass information within the `subpass` field
#[derive(Debug)]
pub struct HashableGraphicsPipelineDescr<T: HashableGraphicsPipelineTypes> {
    pub label: Option<Cow<'static, str>>,

    /// Should contain hashable state for:
    /// * The set of shader programs used
    /// * The derived pipeline layout
    pub program: T::Program,

    /// Should contain hashable state for the render pass and subpass this
    /// graphics pipeline is used within.
    pub subpass: T::Subpass,

    pub primitive_assembler: HashablePrimitiveAssemblerDescr,
    pub rasterizer: pso::Rasterizer,
    pub fragment: bool,
    pub blender: pso::BlendDesc,
    pub depth_stencil: pso::DepthStencilDesc,
    pub multisampling: Option<pso::Multisampling>,
    //pub baked_states: pso::BakedStates,
}

impl<T: HashableGraphicsPipelineTypes> PartialEq for HashableGraphicsPipelineDescr<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.label == rhs.label
            && self.program == rhs.program
            && self.subpass == rhs.subpass
            && self.primitive_assembler == rhs.primitive_assembler
            && self.rasterizer == rhs.rasterizer
            && self.fragment == rhs.fragment
            && self.blender == rhs.blender
            && self.depth_stencil == rhs.depth_stencil
            && self.multisampling == rhs.multisampling
            //&& self.baked_states == rhs.baked_states
    }
}
impl<T: HashableGraphicsPipelineTypes> Eq for HashableGraphicsPipelineDescr<T> {}

impl<T: HashableGraphicsPipelineTypes> Hash for HashableGraphicsPipelineDescr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.label.hash(state);
        self.program.hash(state);
        self.subpass.hash(state);
        self.primitive_assembler.hash(state);
        {
            // rasterizer
            let r = &self.rasterizer;
            state.write_u8(r.polygon_mode as u8);
            r.cull_face.hash(state);
            r.front_face.hash(state);
            r.depth_clamping.hash(state);
            match r.depth_bias {
                None => state.write_u8(0),
                Some(pso::State::Dynamic) => state.write_u8(1),
                Some(pso::State::Static(depth_bias)) => {
                    state.write_u8(2);
                    depth_bias.const_factor.to_be_bytes().hash(state);
                    depth_bias.clamp.to_be_bytes().hash(state);
                    depth_bias.slope_factor.to_be_bytes().hash(state);
                },
            }
            r.conservative.hash(state);
            match r.line_width {
                pso::State::Dynamic => state.write_u8(0),
                pso::State::Static(width) => {
                    state.write_u8(1);
                    width.to_be_bytes().hash(state);
                },
            }
        }
        self.fragment.hash(state);
        {
            // blender
            let b = &self.blender;
            match b.logic_op.clone() {
                None => state.write_u8(0),
                Some(op) => {
                    state.write_u8(1);
                    state.write_u8(op as u8);
                },
            }
            b.targets.hash(state);
        }
        self.depth_stencil.hash(state);
        match &self.multisampling {
            None => state.write_u8(0),
            Some(m) => {
                state.write_u8(1);
                m.rasterization_samples.hash(state);
                match m.sample_shading {
                    None => state.write_u8(0),
                    Some(s) => {
                        state.write_u8(1);
                        s.to_be_bytes().hash(state);
                    },
                }
                m.sample_mask.hash(state);
                m.alpha_coverage.hash(state);
                m.alpha_to_one.hash(state);
            },
        }
        //{
        //    // baked states
        //    let b = &self.baked_states;
        //    match &b.viewport {
        //        None => state.write_u8(0),
        //        Some(v) => {
        //            state.write_u8(1);
        //            v.rect.hash(state);
        //            v.depth.start.to_be_bytes().hash(state);
        //            v.depth.end.to_be_bytes().hash(state);
        //        },
        //    }
        //    b.scissor.hash(state);
        //    match b.blend_color {
        //        None => state.write_u8(0),
        //        Some(bc) => for sc in bc.iter() {
        //            sc.to_be_bytes().hash(state);
        //        },
        //    }
        //    match &b.depth_bounds {
        //        None => state.write_u8(0),
        //        Some(range) => {
        //            state.write_u8(1);
        //            range.start.to_be_bytes().hash(state);
        //            range.end.to_be_bytes().hash(state);
        //        },
        //    }
        //}
    }
}
