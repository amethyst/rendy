use super::submission::{Submission, SubmissionId};

/// Queue id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueueId {
    /// Family id of the queue.
    pub family: rendy_core::hal::queue::QueueFamilyId,

    /// Index of the queue.
    pub index: usize,
}

impl QueueId {
    /// Create queue id from family id and index.
    pub fn new(family: rendy_core::hal::queue::QueueFamilyId, index: usize) -> Self {
        QueueId { family, index }
    }

    /// Get family id.
    pub fn family(&self) -> rendy_core::hal::queue::QueueFamilyId {
        self.family
    }

    /// Get index within the family.
    pub fn index(&self) -> usize {
        self.index
    }
}

/// Instances of this type contains array of `Submission`s.
/// Those submissions are expected to be submitted in order.
#[derive(Clone, Debug)]
pub struct Queue<S> {
    id: QueueId,
    submissions: Vec<Submission<S>>,
}

impl<S> Queue<S> {
    /// Create new queue with specified id.
    pub fn new(id: QueueId) -> Self {
        Queue {
            id,
            submissions: Vec::new(),
        }
    }

    /// Get id of the queue.
    pub fn id(&self) -> QueueId {
        self.id
    }

    /// Iterate over immutable references to each submission in this queue
    pub fn iter(&self) -> impl Iterator<Item = &Submission<S>> {
        self.submissions.iter()
    }

    /// Iterate over mutable references to each submission in this queue
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Submission<S>> {
        self.submissions.iter_mut()
    }

    // /// Iterate over mutable references to each submission in this queue
    // pub fn into_iter(self) -> QueueIntoIter<S> {
    //     QueueIntoIter {
    //         qid: self.id,
    //         iter: self.submissions.into_iter().enumerate(),
    //     }
    // }

    /// Get the number of submissions in queue.
    pub fn len(&self) -> usize {
        self.submissions.len()
    }

    /// Get reference to `Submission` instance by id.
    ///
    /// # Panic
    ///
    /// This function will panic if requested submission isn't part of this queue.
    ///
    pub fn submission(&self, sid: SubmissionId) -> Option<&Submission<S>> {
        self.submissions.get(sid.index())
    }

    /// Get mutable reference to `Submission` instance by id.
    ///
    /// # Panic
    ///
    /// This function will panic if requested submission isn't part of this queue.
    ///
    pub fn submission_mut(&mut self, sid: SubmissionId) -> Option<&mut Submission<S>> {
        self.submissions.get_mut(sid.index())
    }

    // /// Get reference to last `Submission` instance.
    // pub fn last_submission(&self) -> Option<&Submission<S>> {
    //     self.submissions.last()
    // }

    // /// Get mutable reference to last `Submission` instance.
    // pub fn last_submission_mut(&mut self) -> Option<&mut Submission<S>> {
    //     self.submissions.last_mut()
    // }

    /// Add `Submission` instance to the end of queue.
    /// Returns id of the added submission.
    pub fn add_submission(
        &mut self,
        node: usize,
        wait_factor: usize,
        submit_order: usize,
        sync: S,
    ) -> SubmissionId {
        let sid = SubmissionId::new(self.id, self.submissions.len());
        self.submissions
            .push(Submission::new(node, wait_factor, submit_order, sid, sync));
        sid
    }

    /// Add `Submission` instance to the end of queue.
    /// Check that submission has correct id.
    pub fn add_submission_checked(&mut self, submission: Submission<S>) {
        assert_eq!(
            submission.id(),
            SubmissionId::new(self.id, self.submissions.len())
        );
        self.submissions.push(submission);
    }
}
