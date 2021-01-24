use rendy_core::hal;

use crate::{
    node::State,
    resource::{AccessFlags, Resource},
    schedule::{QueueId, SubmissionId},
};

/// State of the link associated with queue.
/// Contains submissions range, combined access and stages bits by submissions from the range.
#[derive(Clone, Copy, Debug)]
pub struct LinkQueueState<R: Resource> {
    pub first: usize,
    pub last: usize,
    pub access: R::Access,
    pub stages: hal::pso::PipelineStage,
}

impl<R> LinkQueueState<R>
where
    R: Resource,
{
    fn new(node: &LinkNode<R>) -> Self {
        LinkQueueState {
            first: node.sid.index(),
            last: node.sid.index(),
            access: node.state.access,
            stages: node.state.stages,
        }
    }

    fn push(&mut self, node: &LinkNode<R>) {
        self.access |= node.state.access;
        self.stages |= node.state.stages;
        self.last = node.sid.index();
    }
}

/// This type defines what states resource are at some point in time when commands recorded into
/// corresponding submissions are executed.
/// Those commands doesn't required to perform actions with all access types declared by the link.
/// But performing actions with access types not declared by the link is prohibited.
#[derive(Clone, Debug)]
pub struct Link<R: Resource> {
    /// Combination of all accesses.
    access: R::Access,

    /// Combination of all usages.
    usage: R::Usage,

    /// Common layout for all submissions.
    layout: R::Layout,

    /// Combination of all stages.
    stages: hal::pso::PipelineStage,

    /// Number of queues involved.
    queue_count: usize,

    /// State per queue.
    queues: Vec<Option<LinkQueueState<R>>>,

    /// Family of queues.
    family: hal::queue::QueueFamilyId,
}

/// Node for the link.
#[derive(Debug)]
pub struct LinkNode<R: Resource> {
    /// Submission id of the node.
    pub sid: SubmissionId,

    /// Resource state of the node.
    pub state: State<R>,
}

impl<R> Link<R>
where
    R: Resource,
{
    /// Create new link with first attached submission.
    ///
    /// # Parameters
    ///
    /// `access`    - Access flags performed in submission.
    /// `usage`     - Usage flags required by submission.
    pub fn new(node: LinkNode<R>) -> Self {
        let mut link = Link {
            access: node.state.access,
            usage: node.state.usage,
            layout: node.state.layout,
            stages: node.state.stages,
            queue_count: 1,
            queues: Vec::new(),
            family: node.sid.family(),
        };
        link.ensure_queue(node.sid.queue().index());
        link.queues[node.sid.queue().index()] = Some(LinkQueueState::new(&node));
        link
    }

    fn ensure_queue(&mut self, index: usize) {
        if index >= self.queues.len() {
            let reserve = index - self.queues.len() + 1;
            self.queues.reserve(reserve);
            while index >= self.queues.len() {
                self.queues.push(None);
            }
        }
    }

    /// Get queue family that owns the resource at the link.
    /// All associated submissions must be from the same queue family.
    pub fn family(&self) -> hal::queue::QueueFamilyId {
        self.family
    }

    /// Get usage.
    pub fn submission_state(&self, sid: SubmissionId) -> State<R> {
        let lqs = self.queues[sid.queue().index()].as_ref().unwrap();
        State {
            access: lqs.access,
            layout: self.layout,
            stages: lqs.stages,
            usage: self.usage,
        }
    }

    /// Get usage.
    pub fn state(&self) -> State<R> {
        State {
            access: self.access,
            layout: self.layout,
            stages: self.stages,
            usage: self.usage,
        }
    }

    /// Get access.
    pub fn access(&self) -> R::Access {
        self.access
    }

    /// Get layout.
    pub fn layout(&self) -> R::Layout {
        self.layout
    }

    /// Get usage.
    pub fn usage(&self) -> R::Usage {
        self.usage
    }

    // /// Get usage.
    // pub fn stages(&self) -> hal::pso::PipelineStage {
    //     self.stages
    // }

    /// Check if the link is associated with only one queue.
    pub fn single_queue(&self) -> bool {
        self.queue_count == 1
    }

    /// Check if the given state and submission are compatible with link.
    /// If compatible then the submission can be associated with the link.
    pub fn compatible(&self, node: &LinkNode<R>) -> bool {
        // If queue the same and states are compatible.
        self.family == node.sid.family() && !(self.access | node.state.access).exclusive()
    }

    /// Insert submission with specified state to the link.
    /// It must be compatible.
    /// Associating submission with the link will allow the submission
    /// to be executed in parallel with other submissions associated with this link.
    /// Unless other chains disallow.
    ///
    /// # Panics
    ///
    /// This function will panic if `state` and `sid` are not compatible.
    /// E.g. `Link::compatible` didn't return `true` for the arguments.
    pub fn add_node(&mut self, node: LinkNode<R>) {
        self.ensure_queue(node.sid.queue().index());

        self.access |= node.state.access;
        self.usage |= node.state.usage;
        self.stages |= node.state.stages;

        match &mut self.queues[node.sid.queue().index()] {
            &mut Some(ref mut queue) => {
                queue.push(&node);
            }
            slot @ &mut None => {
                self.queue_count += 1;
                *slot = Some(LinkQueueState::new(&node));
            }
        }
    }

    // /// Check if ownership transfer is required between those links.
    // pub fn transfer_required(&self, next: &Self) -> bool {
    //     self.family != next.family
    // }

    /// Iterate over queues.
    pub fn queues(&self) -> impl Iterator<Item = (QueueId, &LinkQueueState<R>)> {
        let family = self.family;
        self.queues
            .iter()
            .enumerate()
            .filter_map(move |(index, queue)| {
                queue
                    .as_ref()
                    .map(move |queue| (QueueId::new(family, index), queue))
            })
    }

    /// Get particular queue
    pub fn queue(&self, qid: QueueId) -> &LinkQueueState<R> {
        self.queues[qid.index()].as_ref().unwrap()
    }
}
