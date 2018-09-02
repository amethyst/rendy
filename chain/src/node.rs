
use std::collections::hash_map::{HashMap, Iter as HashMapIter};

use schedule::FamilyId;
use access::AccessFlags;
use resource::{Buffer, Image, Resource};
use stage::PipelineStageFlags;
use Id;

/// State in which node uses resource and usage flags.
#[derive(Clone, Copy, Debug)]
pub struct State<R: Resource> {
    pub access: AccessFlags,
    pub layout: R::Layout,
    pub stages: PipelineStageFlags,
    pub usage: R::Usage,
}

/// Description of node.
#[derive(Clone, Debug)]
pub struct Node {
    /// Id of the node.
    pub id: usize,

    /// Family required to execute the node.
    pub family: FamilyId,

    /// Specific queue for the node. Or `None` if irrelevant.
    pub queue: Option<usize>,

    /// Dependencies of the node.
    /// Those are indices of other nodees in array.
    pub dependencies: Vec<usize>,

    /// Buffer category ids and required state.
    pub buffers: HashMap<Id, State<Buffer>>,

    /// Image category ids and required state.
    pub images: HashMap<Id, State<Image>>,
}

impl Node {
    /// Get family on which this node will be executed.
    pub fn family(&self) -> FamilyId {
        self.family
    }

    /// Get queue to which this node assigned. Or `None`.
    pub fn queue(&self) -> Option<usize> {
        self.queue
    }

    /// Get indices of nodees this node depends on.
    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    /// Get iterator to buffer states this node accesses.
    pub fn buffers(&self) -> HashMapIter<Id, State<Buffer>> {
        self.buffers.iter()
    }

    /// Get iterator to image states this node accesses.
    pub fn images(&self) -> HashMapIter<Id, State<Image>> {
        self.images.iter()
    }
}
