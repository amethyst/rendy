use super::{
    queue::{Queue, QueueId},
    submission::{Submission, SubmissionId},
};

/// Instances of this type contains array of `Queue`s.
/// All contained queues has identical capabilities.
#[derive(Clone, Debug)]
pub struct Family<S> {
    id: rendy_core::hal::queue::QueueFamilyId,
    queues: Vec<Queue<S>>,
}

impl<S> Family<S> {
    /// Create new empty `Family`
    pub fn new(id: rendy_core::hal::queue::QueueFamilyId) -> Self {
        Family {
            id,
            queues: Vec::default(),
        }
    }

    /// Get id of the family.
    pub fn id(&self) -> rendy_core::hal::queue::QueueFamilyId {
        self.id
    }

    /// Get reference to `Queue` instance by the id.
    ///
    /// # Panic
    ///
    /// This function will panic if requested queue isn't part of this family.
    ///
    pub fn queue(&self, qid: QueueId) -> Option<&Queue<S>> {
        assert_eq!(self.id, qid.family());
        self.queues.get(qid.index())
    }

    /// Get mutable reference to `Queue` instance by the id.
    ///
    /// # Panic
    ///
    /// This function will panic if requested queue isn't part of this family.
    ///
    pub fn queue_mut(&mut self, qid: QueueId) -> Option<&mut Queue<S>> {
        assert_eq!(self.id, qid.family());
        self.queues.get_mut(qid.index())
    }

    /// Get mutable reference to `Queue` instance by the id.
    /// This function will grow queues array if index is out of bounds.
    ///
    /// # Panic
    ///
    /// This function will panic if requested queue isn't part of this family.
    ///
    pub fn ensure_queue(&mut self, qid: QueueId) -> &mut Queue<S> {
        assert_eq!(self.id, qid.family());
        let len = self.queues.len();
        self.queues
            .extend((len..qid.index() + 1).map(|i| Queue::new(QueueId::new(qid.family(), i))));
        &mut self.queues[qid.index()]
    }

    /// Get reference to `Submission<S>` instance by id.
    ///
    /// # Panic
    ///
    /// This function will panic if requested submission isn't part of this family.
    ///
    pub fn submission(&self, sid: SubmissionId) -> Option<&Submission<S>> {
        assert_eq!(self.id, sid.family());
        self.queue(sid.queue())
            .and_then(|queue| queue.submission(sid))
    }

    /// Get mutable reference to `Submission<S>` instance by id.
    ///
    /// # Panic
    ///
    /// This function will panic if requested submission isn't part of this family.
    ///
    pub fn submission_mut(&mut self, sid: SubmissionId) -> Option<&mut Submission<S>> {
        assert_eq!(self.id, sid.family());
        self.queue_mut(sid.queue())
            .and_then(|queue| queue.submission_mut(sid))
    }

    /// Iterate over queues.
    pub fn iter(&self) -> impl Iterator<Item = &Queue<S>> {
        self.queues.iter()
    }

    /// Iterate over queues.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Queue<S>> {
        self.queues.iter_mut()
    }

    /// The number of queues in this schedule.
    pub fn queue_count(&self) -> usize {
        self.queues.len()
    }
}
