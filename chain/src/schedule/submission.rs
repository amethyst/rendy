use std::collections::HashMap;
use super::queue::QueueId;
use crate::Id;

/// Submission id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubmissionId {
    /// Queue id of the submission.
    pub queue: QueueId,

    /// Index of the queue.
    pub index: usize,
}

impl SubmissionId {
    /// Create new id from queue id and index.
    pub fn new(queue: QueueId, index: usize) -> Self {
        SubmissionId { queue, index }
    }

    /// Get family id.
    pub fn family(&self) -> gfx_hal::queue::QueueFamilyId {
        self.queue.family()
    }

    /// Get queue id.
    pub fn queue(&self) -> QueueId {
        self.queue
    }

    /// Get index.
    pub fn index(&self) -> usize {
        self.index
    }
}

/// This type corresponds to commands that should be recorded into single primary command buffer.
#[derive(Clone, Debug)]
pub struct Submission<S> {
    node: usize,
    id: SubmissionId,
    buffer_links: HashMap<Id, usize>,
    image_links: HashMap<Id, usize>,
    wait_factor: usize,
    submit_order: usize,
    sync: S,
}

impl<S> Submission<S> {
    /// Get id of the `Node`.
    pub fn node(&self) -> usize {
        self.node
    }

    /// Get id of the `Submission`.
    pub fn id(&self) -> SubmissionId {
        self.id
    }

    /// Get synchronization for `Submission`.
    pub fn sync(&self) -> &S {
        &self.sync
    }

    /// Get wait factor for `Submission`
    pub fn wait_factor(&self) -> usize {
        self.wait_factor
    }

    /// Get submit order for `Submission`
    pub fn submit_order(&self) -> usize {
        self.submit_order
    }

    /// Get link index for resource by id.
    pub fn buffer_link_index(&self, id: Id) -> usize {
        self.buffer_links[&id]
    }

    /// Set link index for given chain.
    pub fn set_buffer_link(&mut self, id: Id, link: usize) {
        assert!(self.buffer_links.insert(id, link).is_none());
    }

    /// Get link index for resource by id.
    pub fn image_link_index(&self, id: Id) -> usize {
        self.image_links[&id]
    }

    /// Set link index for given chain.
    pub fn set_image_link(&mut self, id: Id, link: usize) {
        assert!(self.image_links.insert(id, link).is_none());
    }

    /// Create new submission with specified pass.
    pub(crate) fn new(
        node: usize,
        wait_factor: usize,
        submit_order: usize,
        id: SubmissionId,
        sync: S,
    ) -> Self {
        Submission {
            node,
            buffer_links: HashMap::default(),
            image_links: HashMap::default(),
            id,
            wait_factor,
            submit_order,
            sync,
        }
    }

    /// Set synchronization to the `Submission`.
    pub fn set_sync<T>(&self, sync: T) -> Submission<T> {
        Submission {
            node: self.node,
            buffer_links: self.buffer_links.clone(),
            image_links: self.image_links.clone(),
            id: self.id,
            wait_factor: self.wait_factor,
            submit_order: self.submit_order,
            sync,
        }
    }
}
