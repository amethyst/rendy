//! This module defines types for execution hierarchy.
//! `Submission` is a piece of work that can be recorded into single primary command buffer.
//! `Submission` contains references to links and semaphores required to wait/signal.
//! `Queue` contains array of `Submission`'s. User is expected to submission corresponding command buffers in the order.
//! `Queue`'s are grouped into `Family`. All queues from one `Family` has identical capabilities.
//! `Schedule` is a set or `Family` instances.
use rendy_core::hal;

mod family;
mod queue;
mod submission;

use std::collections::HashMap;
use std::ops::{Index, IndexMut};

pub use self::{
    family::Family,
    queue::{Queue, QueueId},
    submission::{Submission, SubmissionId},
};

use derivative::Derivative;

/// Whole passes schedule.
#[derive(Clone, Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct Schedule<S> {
    map: HashMap<hal::queue::QueueFamilyId, Family<S>>,
    ordered: Vec<SubmissionId>,
}

impl<S> Schedule<S> {
    /// Get total number of submissions.
    pub fn total(&self) -> usize {
        self.ordered.len()
    }

    /// Iterate over submissions in ordered they must be submitted.
    pub fn ordered(&self) -> impl Iterator<Item = &Submission<S>> {
        let map = &self.map;

        self.ordered
            .iter()
            .map(move |&sid| map[&sid.family()].submission(sid).unwrap())
    }

    /// The number of families in this schedule.
    pub fn family_count(&self) -> usize {
        self.map.len()
    }

    /// The number of queues in this schedule.
    pub fn queue_count(&self) -> usize {
        self.map.iter().map(|x| x.1.queue_count()).sum()
    }

    /// Iterate over immutable references to families in this schedule.
    pub fn iter(&self) -> impl Iterator<Item = &Family<S>> {
        self.map.values()
    }

    /// Iterate over mutable references to families in this schedule
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Family<S>> {
        self.map.values_mut()
    }

    /// Get reference to `Family` instance by the id.
    pub fn family(&self, fid: hal::queue::QueueFamilyId) -> Option<&Family<S>> {
        self.map.get(&fid)
    }

    /// Get mutable reference to `Family` instance by the id.
    pub fn family_mut(&mut self, fid: hal::queue::QueueFamilyId) -> Option<&mut Family<S>> {
        self.map.get_mut(&fid)
    }

    /// Get reference to `Queue` instance by the id.
    pub fn queue(&self, qid: QueueId) -> Option<&Queue<S>> {
        self.family(qid.family())
            .and_then(|family| family.queue(qid))
    }

    /// Get mutable reference to `Queue` instance by the id.
    pub fn queue_mut(&mut self, qid: QueueId) -> Option<&mut Queue<S>> {
        self.family_mut(qid.family())
            .and_then(|family| family.queue_mut(qid))
    }

    /// Get reference to `Submission` instance by id.
    pub fn submission(&self, sid: SubmissionId) -> Option<&Submission<S>> {
        self.queue(sid.queue())
            .and_then(|queue| queue.submission(sid))
    }

    /// Get reference to `Submission` instance by id.
    pub fn submission_mut(&mut self, sid: SubmissionId) -> Option<&mut Submission<S>> {
        self.queue_mut(sid.queue())
            .and_then(|queue| queue.submission_mut(sid))
    }

    /// Get mutable reference to `Family` instance by the id.
    /// This function will add empty `Family` if id is not present.
    pub fn ensure_family(&mut self, fid: hal::queue::QueueFamilyId) -> &mut Family<S> {
        self.map.entry(fid).or_insert_with(|| Family::new(fid))
    }

    /// Get mutable reference to `Queue` instance by the id.
    /// This function will grow queues array if index is out of bounds.
    pub fn ensure_queue(&mut self, qid: QueueId) -> &mut Queue<S> {
        self.ensure_family(qid.family()).ensure_queue(qid)
    }

    /// Set queue to the schedule.
    pub(crate) fn set_queue(&mut self, queue: Queue<S>) {
        let qid = queue.id();
        *self.ensure_queue(qid) = queue;
    }

    /// Make ordered.
    pub fn build_order(&mut self) {
        let mut oredered: Vec<SubmissionId> = Vec::new();

        {
            let submissions = self
                .iter()
                .flat_map(|family| family.iter().flat_map(|queue| queue.iter()));

            for submission in submissions {
                if submission.submit_order() == !0 {
                    continue;
                }
                let len = oredered.len();
                if len <= submission.submit_order() {
                    oredered.extend((len..submission.submit_order() + 1).map(|_| submission.id()));
                } else {
                    oredered[submission.submit_order()] = submission.id();
                }
            }
        }

        self.ordered = oredered;
    }
}

impl<S> Index<QueueId> for Schedule<S> {
    type Output = Queue<S>;

    fn index(&self, qid: QueueId) -> &Queue<S> {
        self.queue(qid).unwrap()
    }
}

impl<S> IndexMut<QueueId> for Schedule<S> {
    fn index_mut(&mut self, qid: QueueId) -> &mut Queue<S> {
        self.queue_mut(qid).unwrap()
    }
}

impl<S> Index<SubmissionId> for Schedule<S> {
    type Output = Submission<S>;

    fn index(&self, sid: SubmissionId) -> &Submission<S> {
        self.submission(sid).unwrap()
    }
}

impl<S> IndexMut<SubmissionId> for Schedule<S> {
    fn index_mut(&mut self, sid: SubmissionId) -> &mut Submission<S> {
        self.submission_mut(sid).unwrap()
    }
}
