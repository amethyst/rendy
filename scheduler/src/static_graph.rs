//! Graph builder which is built once, then constructed and run once for every frame.
//!
//! An alternative builder to this could store the nodes themselves outside of the graph,
//! and build the graph on every frame.
//! This other (more powerful) approach would NOT be that much more expensive at runtime,
//! but this builder is simpler to use, and likely preferred if you do not need the extra
//! power the other builder could provide.
//!
use std::collections::{BTreeSet, BTreeMap};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use {
    crate::{
        chain,
        command::{Families, FamilyId, QueueId},
        core::{device_owned, DeviceId},
        factory::Factory,
        frame::{Fences, Frame, Frames},
        memory::Data,
        node::{
            BufferBarrier, DynNode, ImageBarrier, NodeBuffer, NodeBuildError, NodeBuilder,
            NodeImage,
        },
        resource::{
            Buffer, BufferCreationError, BufferInfo, Handle, Image, ImageCreationError, ImageInfo,
        },
        BufferId, ImageId, NodeId,
    },
    rendy_core::hal::{queue::QueueFamilyId, Backend},
    thread_profiler::profile_scope,
};

use super::{
    DynamicParameter, Parameter,
    DynNodeConstructor, NodeConstructor,
    Graph,
};

/// Type to enforce that creation of graph resources are only performed within the context
/// of a node.
/// Derefs to GraphBuilder, so should be pretty transparrent.
pub struct GraphNodeBuilder<'a, B: Backend> {
    node: NodeId,
    graph: &'a mut GraphBuilder<B>,
    current_deps: Option<BTreeSet<DynamicParameter>>,
}
impl<'a, B: Backend> GraphNodeBuilder<'a, B> {
    fn new(gb: &'a mut GraphBuilder<B>, node: NodeId) -> Self {
        Self {
            node,
            graph: gb,
            current_deps: Some(BTreeSet::new()),
        }
    }

    /// Adds a parameter that the current node produces.
    pub fn add_parameter<P: 'static>(&mut self) -> Parameter<P> {
        let node = self.node;
        let parameter = Parameter(PhantomData, self.parameters.len());
        self.parameters.push(ParameterData {
            parameter: parameter.into(),
            producer: node,
        });
        parameter
    }

    /// Adds a parameter that the current node uses. This introduces a dependency on the producer of the parameter.
    pub fn use_parameter<P: Into<DynamicParameter>>(&mut self, parameter: P) {
        let param: DynamicParameter = parameter.into();
        let node = self.node;
        self.current_deps.as_mut().unwrap().insert(param);

        let other_node = self.parameters[param.1].producer;
        self.nodes[other_node.0].dependents.insert(node);
    }
}
impl<'a, B: Backend> Deref for GraphNodeBuilder<'a, B> {
    type Target = GraphBuilder<B>;
    fn deref(&self) -> &Self::Target {
        self.graph
    }
}
impl<'a, B: Backend> DerefMut for GraphNodeBuilder<'a, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.graph
    }
}

pub(crate) struct ParameterData {
    parameter: DynamicParameter,
    producer: NodeId,
}

pub(crate) struct NodeData<B: Backend> {
    id: NodeId,
    node: Box<dyn DynNodeConstructor<B>>,

    dependents: BTreeSet<NodeId>,
    dependencies: BTreeSet<DynamicParameter>,
}

pub struct GraphBuilder<B: Backend> {
    nodes: Vec<NodeData<B>>,
    parameters: Vec<ParameterData>,
}

impl<B: Backend> GraphBuilder<B> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            parameters: Vec::new(),
        }
    }

    /// Adds the given node builder to the stack, returning the added resources.
    pub fn add<NB: NodeConstructor<B> + DynNodeConstructor<B> + 'static>(&mut self, mut node_builder: NB) -> NB::Outputs {
        let node_id = NodeId(self.nodes.len());
        let (outputs, deps) = {
            let mut gnb = GraphNodeBuilder::new(self, node_id);
            (
                node_builder.register_outputs(&mut gnb),
                gnb.current_deps.take().unwrap(),
            )
        };
        self.nodes.push(NodeData {
            id: node_id,
            node: Box::new(node_builder),

            dependencies: deps,
            dependents: BTreeSet::new(),
        });
        outputs
    }

    fn topological_sort(&self) -> Vec<NodeId> {
        let mut out = Vec::new();

        let edges: BTreeSet<_> = {
            let map_closure = |b: &DynamicParameter| self.parameters[b.1].producer;
            self.nodes
                .iter()
                .flat_map(|n| n.dependencies.iter().map(map_closure).zip(std::iter::repeat(n.id)))
                .collect()
        };

        let mut root_nodes: BTreeSet<_> = self
            .nodes
            .iter()
            .filter(|n| n.dependencies.len() == 0)
            .map(|n| n.id)
            .collect();
        let mut used_edges = BTreeSet::<(NodeId, NodeId)>::new();

        while root_nodes.len() > 0 {
            let node = *root_nodes.iter().next().unwrap();
            root_nodes.remove(&node);

            out.push(node);

            for edge_target in self.nodes[node.0].dependents.iter() {
                //let edge_target = self.parameters[edge_target_param.1].producer;
                let edge = (node, *edge_target);
                if used_edges.contains(&edge) {
                    continue;
                }

                used_edges.insert(edge);

                let has_incoming = self.nodes[edge_target.0]
                    .dependencies
                    .iter()
                    .map(|n| self.parameters[n.1].producer)
                    .all(|n| !used_edges.contains(&(n, node)));

                if !has_incoming {
                    root_nodes.insert(*edge_target);
                }
            }
        }

        if edges != used_edges {
            panic!("Graph contains cycles");
        }

        out
    }

    pub fn build(self) -> Graph<B> {
        let schedule = self.topological_sort();

        Graph {
            schedule,
            nodes: self.nodes,
            parameters: self.parameters,
        }
    }
}
