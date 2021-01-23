use rendy_core::hal;
use std::collections::hash_map::{HashMap, Iter as HashMapIter};

use crate::{
    resource::{Buffer, Image, Resource},
    Id,
};

/// State in which node uses resource and usage flags.
#[derive(Clone, Copy, Debug)]
pub struct State<R: Resource> {
    /// Access performed by the node.
    pub access: R::Access,

    /// Optional layout in which node can use resource.
    pub layout: R::Layout,

    /// Stages at which resource is accessed.
    pub stages: hal::pso::PipelineStage,

    /// Usage flags required for resource.
    pub usage: R::Usage,
}

/// Type alias for `State<Buffer>`
pub type BufferState = State<Buffer>;

/// Type alias for `State<Image>`
pub type ImageState = State<Image>;

/// Description of node.
#[derive(Clone, Debug)]
pub struct Node {
    /// Id of the node.
    pub id: usize,

    /// Family required to execute the node.
    pub family: hal::queue::QueueFamilyId,

    /// Dependencies of the node.
    /// Those are indices of other nodes in array.
    pub dependencies: Vec<usize>,

    /// Buffer category ids and required state.
    pub buffers: HashMap<Id, State<Buffer>>,

    /// Image category ids and required state.
    pub images: HashMap<Id, State<Image>>,
}

impl Node {
    /// Get family on which this node will be executed.
    pub fn family(&self) -> hal::queue::QueueFamilyId {
        self.family
    }

    /// Get indices of nodes this node depends on.
    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    /// Get iterator to buffer states this node accesses.
    pub fn buffers(&self) -> HashMapIter<'_, Id, State<Buffer>> {
        self.buffers.iter()
    }

    /// Get iterator to image states this node accesses.
    pub fn images(&self) -> HashMapIter<'_, Id, State<Image>> {
        self.images.iter()
    }
}
