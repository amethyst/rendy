use fnv::FnvHashMap;

use super::{family::FamilyId, queue::QueueId};
use Id;

/// Submission id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubmissionId {
    pub queue: QueueId,
    pub index: usize,
}

impl SubmissionId {
    /// Create new id from queue id and index.
    pub fn new(queue: QueueId, index: usize) -> Self {
        SubmissionId { queue, index }
    }

    /// Get family id.
    pub fn family(&self) -> FamilyId {
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
    id: SubmissionId,
    resource_links: FnvHashMap<Id, usize>,
    wait_factor: usize,
    submit_order: usize,
    sync: S,
}

impl<S> Submission<S> {
    /// Get synchronization for `Submission`.
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

    /// Get link index for buffer by id.
    pub fn resource_link_index(&self, id: Id) -> usize {
        self.resource_links[&id]
    }

    /// Create new submission with specified pass.
    pub(crate) fn new(wait_factor: usize, submit_order: usize, id: SubmissionId, sync: S) -> Self {
        Submission {
            resource_links: FnvHashMap::default(),
            id,
            wait_factor,
            submit_order,
            sync,
        }
    }

    /// Set synchronization to the `Submission`.
    pub(crate) fn set_sync<T>(&self, sync: T) -> Submission<T> {
        Submission {
            resource_links: self.resource_links.clone(),
            id: self.id,
            wait_factor: self.wait_factor,
            submit_order: self.submit_order,
            sync,
        }
    }

    pub(crate) fn set_link(&mut self, id: Id, link: usize) {
        self.resource_links.insert(id, link);
    }
}
