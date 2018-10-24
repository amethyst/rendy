//! This module provide functions for find all required synchronizations (barriers and semaphores).
//!

use std::ops::{Range, RangeFrom, RangeTo};

use fnv::FnvHashMap;
use ash::vk::{AccessFlags, PipelineStageFlags};

use chain::{Chain, Link};
use collect::Chains;
use node::State;
use resource::{Buffer, Image, Resource};
use schedule::{Queue, QueueId, Schedule, SubmissionId};
use Id;

/// Semaphore identifier.
/// It allows to distinguish different semaphores to be later replaced in `Signal`s and `Wait`s
/// for references to semaphores (or tokens associated with real semaphores).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Semaphore {
    id: Id,
    points: Range<SubmissionId>,
}

impl Semaphore {
    fn new(id: Id, points: Range<SubmissionId>) -> Self {
        Semaphore { id, points }
    }
}

/// Semaphore signal info.
/// There must be paired wait.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Signal<S>(S);

impl<S> Signal<S> {
    /// Create signaling for specified point.
    /// At this point `Wait` must be created as well.
    /// `id` and `point` combination must be unique.
    fn new(semaphore: S) -> Self {
        Signal(semaphore)
    }

    /// Get semaphore of the `Signal`.
    pub fn semaphore(&self) -> &S {
        &self.0
    }
}

/// Semaphore wait info.
/// There must be paired signal.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Wait<S>(S, PipelineStageFlags);

impl<S> Wait<S> {
    /// Create waiting for specified point.
    /// At this point `Signal` must be created as well.
    /// `id` and `point` combination must be unique.
    fn new(semaphore: S, stages: PipelineStageFlags) -> Self {
        Wait(semaphore, stages)
    }

    /// Get semaphore of the `Wait`.
    pub fn semaphore(&self) -> &S {
        &self.0
    }

    /// Stage at which to wait.
    pub fn stage(&self) -> PipelineStageFlags {
        self.1
    }
}

/// Pipeline barrier info.
#[derive(Clone, Debug)]
pub struct Barrier<R: Resource> {
    /// `Some` queue for ownership transfer. Or `None`
    pub queues: Option<Range<QueueId>>,

    /// State transition.
    pub states: Range<(AccessFlags, R::Layout, PipelineStageFlags)>,
}

impl<R> Barrier<R>
where
    R: Resource,
{
    fn new(states: Range<State<R>>) -> Self {
        Barrier {
            queues: None,
            states: (
                states.start.access,
                states.start.layout,
                states.start.stages,
            )..(states.end.access, states.end.layout, states.end.stages),
        }
    }

    fn transfer(queues: Range<QueueId>, states: Range<(AccessFlags, R::Layout)>) -> Self {
        Barrier {
            queues: Some(queues),
            states: (
                states.start.0,
                states.start.1,
                PipelineStageFlags::TOP_OF_PIPE,
            )
                ..(
                    states.end.0,
                    states.end.1,
                    PipelineStageFlags::BOTTOM_OF_PIPE,
                ),
        }
    }

    fn acquire(
        queues: Range<QueueId>,
        left: RangeFrom<R::Layout>,
        right: RangeTo<(AccessFlags, R::Layout)>,
    ) -> Self {
        Self::transfer(
            queues,
            (AccessFlags::empty(), left.start)..(right.end.0, right.end.1),
        )
    }

    fn release(
        queues: Range<QueueId>,
        left: RangeFrom<(AccessFlags, R::Layout)>,
        right: RangeTo<R::Layout>,
    ) -> Self {
        Self::transfer(
            queues,
            (left.start.0, left.start.1)..(AccessFlags::empty(), right.end),
        )
    }
}

/// Map of barriers by resource id.
pub type Barriers<R> = FnvHashMap<Id, Barrier<R>>;

/// Map of barriers by buffer id.
pub type BufferBarriers = Barriers<Buffer>;

/// Map of barriers by image id.
pub type ImageBarriers = Barriers<Image>;

/// Synchronization for submission at one side.
#[derive(Clone, Debug)]
pub struct Guard {
    /// Buffer pipeline barriers to be inserted before or after (depends on the side) commands of the submission.
    pub buffers: BufferBarriers,

    /// Image pipeline barriers to be inserted before or after (depends on the side) commands of the submission.
    pub images: ImageBarriers,
}

impl Guard {
    fn new() -> Self {
        Guard {
            buffers: FnvHashMap::default(),
            images: FnvHashMap::default(),
        }
    }

    fn pick<R: Resource>(&mut self) -> &mut Barriers<R> {
        use std::any::Any;
        let Guard {
            ref mut buffers,
            ref mut images,
        } = *self;
        Any::downcast_mut(buffers)
            .or_else(move || Any::downcast_mut(images))
            .expect("`R` should be `Buffer` or `Image`")
    }
}

/// Both sides of synchronization for submission.
#[derive(Clone, Debug)]
pub struct SyncData<S, W> {
    /// Points at other queues that must be waited before commands from the submission can be executed.
    pub wait: Vec<Wait<W>>,

    /// Acquire side of submission synchronization.
    /// Synchronization commands from this side must be recorded before main commands of submission.
    pub acquire: Guard,

    /// Release side of submission synchronization.
    /// Synchronization commands from this side must be recorded after main commands of submission.
    pub release: Guard,

    /// Points at other queues that can run after barriers above.
    pub signal: Vec<Signal<S>>,
}

impl<S, W> SyncData<S, W> {
    fn new() -> Self {
        SyncData {
            wait: Vec::new(),
            acquire: Guard::new(),
            release: Guard::new(),
            signal: Vec::new(),
        }
    }

    fn convert_signal<F, T>(self, mut f: F) -> SyncData<T, W>
    where
        F: FnMut(S) -> T,
    {
        SyncData {
            wait: self.wait,
            acquire: Guard {
                buffers: self.acquire.buffers,
                images: self.acquire.images,
            },
            release: Guard {
                buffers: self.release.buffers,
                images: self.release.images,
            },
            signal: self
                .signal
                .into_iter()
                .map(|Signal(semaphore)| Signal(f(semaphore)))
                .collect(),
        }
    }

    fn convert_wait<F, T>(self, mut f: F) -> SyncData<S, T>
    where
        F: FnMut(W) -> T,
    {
        SyncData {
            wait: self
                .wait
                .into_iter()
                .map(|Wait(semaphore, stage)| Wait(f(semaphore), stage))
                .collect(),
            acquire: Guard {
                buffers: self.acquire.buffers,
                images: self.acquire.images,
            },
            release: Guard {
                buffers: self.release.buffers,
                images: self.release.images,
            },
            signal: self.signal,
        }
    }
}

struct SyncTemp(FnvHashMap<SubmissionId, SyncData<Semaphore, Semaphore>>);
impl SyncTemp {
    fn get_sync(&mut self, sid: SubmissionId) -> &mut SyncData<Semaphore, Semaphore> {
        self.0.entry(sid).or_insert_with(|| SyncData::new())
    }
}

/// Find required synchronization for all submissions in `Chains`.
pub fn sync<F, S, W>(chains: &Chains, mut new_semaphore: F) -> Schedule<SyncData<S, W>>
where
    F: FnMut() -> (S, W),
{
    let ref schedule = chains.schedule;
    let ref buffers = chains.buffers;
    let ref images = chains.images;

    let mut sync = SyncTemp(FnvHashMap::default());
    for (&id, chain) in buffers {
        sync_chain(id, chain, schedule, &mut sync);
    }
    for (&id, chain) in images {
        sync_chain(id, chain, schedule, &mut sync);
    }

    if schedule.queue_count() > 1 {
        optimize(schedule, &mut sync);
    }

    let mut result = Schedule::new();
    let mut signals: FnvHashMap<Semaphore, Option<S>> = FnvHashMap::default();
    let mut waits: FnvHashMap<Semaphore, Option<W>> = FnvHashMap::default();

    for queue in schedule.iter().flat_map(|family| family.iter()) {
        let mut new_queue = Queue::new(queue.id());
        for submission in queue.iter() {
            let sync = if let Some(sync) = sync.0.remove(&submission.id()) {
                let sync = sync.convert_signal(|semaphore| match signals.get_mut(&semaphore) {
                    None => {
                        let (signal, wait) = new_semaphore();
                        let old = waits.insert(semaphore, Some(wait));
                        assert!(old.is_none());
                        signal
                    }
                    Some(signal) => signal.take().unwrap(),
                });
                let sync = sync.convert_wait(|semaphore| match waits.get_mut(&semaphore) {
                    None => {
                        let (signal, wait) = new_semaphore();
                        let old = signals.insert(semaphore, Some(signal));
                        assert!(old.is_none());
                        wait
                    }
                    Some(wait) => wait.take().unwrap(),
                });
                sync
            } else {
                SyncData::new()
            };
            new_queue.add_submission_checked(submission.set_sync(sync));
        }
        result.set_queue(new_queue);
    }

    debug_assert!(sync.0.is_empty());
    debug_assert!(signals.values().all(|x| x.is_none()));
    debug_assert!(waits.values().all(|x| x.is_none()));

    result
}

// submit_order creates a consistent direction in which semaphores are generated, avoiding issues
// with deadlocks.
fn latest<R, S>(link: &Link<R>, schedule: &Schedule<S>) -> SubmissionId
where
    R: Resource,
{
    let (_, sid) = link
        .queues()
        .map(|(qid, queue)| {
            let sid = SubmissionId::new(qid, queue.last);
            (schedule[sid].submit_order(), sid)
        }).max_by_key(|&(submit_order, sid)| (submit_order, sid.queue().index()))
        .unwrap();
    sid
}

fn earliest<R, S>(link: &Link<R>, schedule: &Schedule<S>) -> SubmissionId
where
    R: Resource,
{
    let (_, sid) = link
        .queues()
        .map(|(qid, queue)| {
            let sid = SubmissionId::new(qid, queue.first);
            (schedule[sid].submit_order(), sid)
        }).min_by_key(|&(submit_order, sid)| (submit_order, sid.queue().index()))
        .unwrap();
    sid
}

fn generate_semaphore_pair<R: Resource>(
    sync: &mut SyncTemp,
    id: Id,
    link: &Link<R>,
    range: Range<SubmissionId>,
) {
    if range.start.queue() != range.end.queue() {
        let semaphore = Semaphore::new(id, range.clone());
        sync.get_sync(range.start)
            .signal
            .push(Signal::new(semaphore.clone()));
        sync.get_sync(range.end)
            .wait
            .push(Wait::new(semaphore, link.queue(range.end.queue()).stages));
    }
}

fn sync_chain<R, S>(id: Id, chain: &Chain<R>, schedule: &Schedule<S>, sync: &mut SyncTemp)
where
    R: Resource,
{
    let uid = id.into();
    for (prev_link, link) in chain.links().windows(2).map(|pair| (&pair[0], &pair[1])) {
        if prev_link.family() == link.family() {
            // Prefer to generate barriers on the acquire side, if possible.
            if prev_link.single_queue() && !link.single_queue() {
                let signal_sid = latest(prev_link, schedule);

                // Generate barrier in prev link's last submission.
                sync.get_sync(signal_sid)
                    .release
                    .pick::<R>()
                    .insert(id, Barrier::new(prev_link.state()..link.state()));

                // Generate semaphores between queues in the previous link and the current one.
                for (queue_id, queue) in link.queues() {
                    let head = SubmissionId::new(queue_id, queue.first);
                    generate_semaphore_pair(sync, uid, link, signal_sid..head);
                }
            } else {
                let wait_sid = earliest(link, schedule);

                // Generate semaphores between queues in the previous link and the current one.
                for (queue_id, queue) in prev_link.queues() {
                    let tail = SubmissionId::new(queue_id, queue.last);
                    generate_semaphore_pair(sync, uid, link, tail..wait_sid);
                }

                // Generate barrier in next link's first submission.
                sync.get_sync(wait_sid)
                    .acquire
                    .pick()
                    .insert(id, Barrier::new(prev_link.state()..link.state()));

                if !link.single_queue() {
                    unimplemented!("This case is unimplemented");
                }
            }
        } else {
            let signal_sid = latest(prev_link, schedule);
            let wait_sid = earliest(link, schedule);

            if !prev_link.single_queue() {
                unimplemented!("This case is unimplemented");
            }

            // Generate a semaphore between the signal and wait sides of the transfer.
            generate_semaphore_pair(sync, uid, link, signal_sid..wait_sid);

            // Generate barriers to transfer the resource to another queue.
            sync.get_sync(signal_sid).release.pick::<R>().insert(
                id,
                Barrier::release(
                    signal_sid.queue()..wait_sid.queue(),
                    (prev_link.access(), prev_link.layout())..,
                    ..link.layout(),
                ),
            );
            sync.get_sync(wait_sid).acquire.pick::<R>().insert(
                id,
                Barrier::acquire(
                    signal_sid.queue()..wait_sid.queue(),
                    prev_link.layout()..,
                    ..(link.access(), link.layout()),
                ),
            );

            if !link.single_queue() {
                unimplemented!("This case is unimplemented");
            }
        }
    }
}

fn optimize_submission(
    sid: SubmissionId,
    found: &mut FnvHashMap<QueueId, usize>,
    sync: &mut SyncTemp,
) {
    let mut to_remove = Vec::new();
    if let Some(sync_data) = sync.0.get_mut(&sid) {
        sync_data
            .wait
            .sort_unstable_by_key(|wait| (wait.stage(), wait.semaphore().points.end.index()));
        sync_data.wait.retain(|wait| {
            let start = wait.semaphore().points.start;
            if let Some(synched_to) = found.get_mut(&start.queue()) {
                if *synched_to >= start.index() {
                    to_remove.push(wait.semaphore().clone());
                    return false;
                } else {
                    *synched_to = start.index();
                    return true;
                }
            }
            found.insert(start.queue(), start.index());
            true
        });
    } else {
        return;
    }

    for semaphore in to_remove.drain(..) {
        // Delete signal as well.
        let ref mut signal = sync.0.get_mut(&semaphore.points.start).unwrap().signal;
        let index = signal
            .iter()
            .position(|signal| signal.0 == semaphore)
            .unwrap();
        signal.swap_remove(index);
    }
}

fn optimize<S>(schedule: &Schedule<S>, sync: &mut SyncTemp) {
    for queue in schedule.iter().flat_map(|family| family.iter()) {
        let mut found = FnvHashMap::default();
        for submission in queue.iter() {
            optimize_submission(submission.id(), &mut found, sync);
        }
    }
}
