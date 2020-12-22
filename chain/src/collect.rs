use std::cmp::max;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Range;

use crate::{
    chain::{BufferChains, Chain, ImageChains, Link, LinkNode},
    node::{Node, State},
    resource::{Buffer, Image, Resource},
    schedule::{Queue, QueueId, Schedule, Submission, SubmissionId},
    Id,
};

/// Placeholder for synchronization type.
#[derive(Clone, Copy, Debug)]
pub struct Unsynchronized;

/// Result of node scheduler.
#[derive(Debug)]
pub struct Chains {
    /// Contains submissions for nodes spread among queue schedule.
    pub schedule: Schedule<Unsynchronized>,

    /// Contains all buffer chains.
    pub buffers: BufferChains,

    /// Contains all image chains.
    pub images: ImageChains,
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct Fitness {
    transfers: usize,
    wait_factor: usize,
}

struct ResolvedNode {
    id: usize,
    family: rendy_core::hal::queue::QueueFamilyId,
    queues: Range<usize>,
    rev_deps: Vec<usize>,
    buffers: Vec<(usize, State<Buffer>)>,
    images: Vec<(usize, State<Image>)>,
}

impl Default for ResolvedNode {
    fn default() -> Self {
        ResolvedNode {
            id: 0,
            family: rendy_core::hal::queue::QueueFamilyId(0),
            queues: 0..0,
            rev_deps: Vec::new(),
            buffers: Vec::new(),
            images: Vec::new(),
        }
    }
}

struct ResolvedNodeSet {
    nodes: Vec<ResolvedNode>,
    queues: Vec<QueueId>,
    buffers: Vec<Id>,
    images: Vec<Id>,
}

struct ChainData<R: Resource> {
    chain: Chain<R>,
    last_link_wait_factor: usize,
    current_link_wait_factor: usize,
    current_family: Option<rendy_core::hal::queue::QueueFamilyId>,
}
impl<R: Resource> Default for ChainData<R> {
    fn default() -> Self {
        ChainData {
            chain: Chain::new(),
            last_link_wait_factor: 0,
            current_link_wait_factor: 0,
            current_family: None,
        }
    }
}

struct QueueData {
    queue: Queue<Unsynchronized>,
    wait_factor: usize,
}

/// Calculate automatic `Chains` for nodes.
/// This function tries to find the most appropriate schedule for nodes execution.
pub fn collect<Q>(nodes: Vec<Node>, max_queues: Q) -> Chains
where
    Q: Fn(rendy_core::hal::queue::QueueFamilyId) -> usize,
{
    // Resolve nodes into a form faster to work with.
    let (nodes, mut unscheduled_nodes) = resolve_nodes(nodes, max_queues);
    let mut ready_nodes = Vec::new();

    // Chains.
    let mut images: Vec<ChainData<Image>> = fill(nodes.images.len());
    let mut buffers: Vec<ChainData<Buffer>> = fill(nodes.buffers.len());

    // Schedule
    let mut schedule = Vec::with_capacity(nodes.queues.len());
    for i in 0..nodes.queues.len() {
        schedule.push(QueueData {
            queue: Queue::new(nodes.queues[i]),
            wait_factor: 0,
        });
    }

    for node in &nodes.nodes {
        if unscheduled_nodes[node.id] == 0 {
            ready_nodes.push(node);
        }
    }

    let mut scheduled = 0;
    if nodes.queues.len() == 1 {
        // With a single queue, wait_factor is always the number of scheduled nodes, and
        // transfers is always zero. Thus, we only need dependency resolution.
        while let Some(node) = ready_nodes.pop() {
            schedule_node(
                &mut ready_nodes,
                &mut unscheduled_nodes,
                &nodes,
                node,
                0,
                scheduled,
                scheduled,
                &mut schedule,
                &mut images,
                &mut buffers,
            );
            scheduled += 1;
        }
    } else {
        while !ready_nodes.is_empty() {
            // Among ready nodes find best fit.
            let (fitness, qid, index) = ready_nodes
                .iter()
                .enumerate()
                .map(|(index, &node)| {
                    let (fitness, qid) = fitness(node, &mut images, &mut buffers, &mut schedule);
                    (fitness, qid, index)
                })
                .min()
                .unwrap();

            let node = ready_nodes.swap_remove(index);
            schedule_node(
                &mut ready_nodes,
                &mut unscheduled_nodes,
                &nodes,
                node,
                qid,
                fitness.wait_factor,
                scheduled,
                &mut schedule,
                &mut images,
                &mut buffers,
            );
            scheduled += 1;
        }
    }
    assert_eq!(scheduled, nodes.nodes.len(), "Dependency loop found!");

    Chains {
        schedule: reify_schedule(schedule),
        buffers: reify_chain(&nodes.buffers, buffers),
        images: reify_chain(&nodes.images, images),
    }
}

fn fill<T: Default>(num: usize) -> Vec<T> {
    let mut vec = Vec::with_capacity(num);
    for _ in 0..num {
        vec.push(T::default());
    }
    vec
}

struct LookupBuilder<I: Hash + Eq + Copy> {
    forward: HashMap<I, usize>,
    backward: Vec<I>,
}
impl<I: Hash + Eq + Copy> LookupBuilder<I> {
    fn new() -> LookupBuilder<I> {
        LookupBuilder {
            forward: HashMap::default(),
            backward: Vec::new(),
        }
    }

    fn forward(&mut self, id: I) -> usize {
        if let Some(&id_num) = self.forward.get(&id) {
            id_num
        } else {
            let id_num = self.backward.len();
            self.backward.push(id);
            self.forward.insert(id, id_num);
            id_num
        }
    }
}

fn resolve_nodes<Q>(nodes: Vec<Node>, max_queues: Q) -> (ResolvedNodeSet, Vec<usize>)
where
    Q: Fn(rendy_core::hal::queue::QueueFamilyId) -> usize,
{
    let node_count = nodes.len();

    let mut unscheduled_nodes = fill(nodes.len());
    let mut reified_nodes: Vec<ResolvedNode> = fill(nodes.len());
    let mut node_ids = LookupBuilder::new();
    let mut queues = LookupBuilder::new();
    let mut buffers = LookupBuilder::new();
    let mut images = LookupBuilder::new();

    let s = RandomState::new();
    let mut family_full = HashMap::with_hasher(s);

    for node in nodes {
        let family = node.family;
        if !family_full.contains_key(&family) {
            let count = max_queues(family);
            assert!(count > 0, "Cannot create a family with 0 max queues.");
            for i in 0..count {
                queues.forward(QueueId::new(family, i));
            }

            let full_range = queues.forward(QueueId::new(family, 0))
                ..queues.forward(QueueId::new(family, count - 1)) + 1;
            family_full.insert(family, full_range);
        }

        let id = node_ids.forward(node.id);
        assert!(id < node_count, "Dependency not found."); // This implies a dep is not there.
        let unscheduled_count = node.dependencies.len();

        for dep in node.dependencies {
            // Duplicated dependencies work fine, since they push two rev_deps entries and add two
            // to unscheduled_nodes.
            reified_nodes[node_ids.forward(dep)].rev_deps.push(id);
        }
        unscheduled_nodes[id] = unscheduled_count;

        // We set these manually, and notably, do *not* touch rev_deps.
        reified_nodes[id].id = id;
        reified_nodes[id].family = node.family;
        reified_nodes[id].queues = family_full[&family].clone();
        reified_nodes[id].buffers = node
            .buffers
            .into_iter()
            .map(|(k, v)| (buffers.forward(k), v))
            .collect();
        reified_nodes[id].images = node
            .images
            .into_iter()
            .map(|(k, v)| (images.forward(k), v))
            .collect();
    }

    (
        ResolvedNodeSet {
            nodes: reified_nodes,
            queues: queues.backward,
            buffers: buffers.backward,
            images: images.backward,
        },
        unscheduled_nodes,
    )
}

fn reify_chain<R: Resource>(ids: &[Id], vec: Vec<ChainData<R>>) -> HashMap<Id, Chain<R>> {
    let mut map = HashMap::with_capacity_and_hasher(vec.len(), Default::default());
    for (chain, &i) in vec.into_iter().zip(ids) {
        map.insert(i, chain.chain);
    }
    map
}

fn reify_schedule(vec: Vec<QueueData>) -> Schedule<Unsynchronized> {
    let mut schedule = Schedule::new();
    for queue_data in vec.into_iter() {
        schedule.set_queue(queue_data.queue);
    }
    schedule
}

fn fitness(
    node: &ResolvedNode,
    images: &mut Vec<ChainData<Image>>,
    buffers: &mut Vec<ChainData<Buffer>>,
    schedule: &mut Vec<QueueData>,
) -> (Fitness, usize) {
    let mut transfers = 0;
    let mut wait_factor_from_chains = 0;

    // Collect minimal waits required and resource transfers count.
    for &(id, _) in &node.buffers {
        let chain = &buffers[id];
        if chain
            .current_family
            .map_or(false, |family| family != node.family)
        {
            transfers += 1;
        }
        wait_factor_from_chains = max(wait_factor_from_chains, chain.last_link_wait_factor);
    }
    for &(id, _) in &node.images {
        let chain = &images[id];
        if chain
            .current_family
            .map_or(false, |family| family != node.family)
        {
            transfers += 1;
        }
        wait_factor_from_chains = max(wait_factor_from_chains, chain.last_link_wait_factor);
    }

    // Find best queue for node.
    let (wait_factor_from_queue, queue) = node
        .queues
        .clone()
        .map(|index| (schedule[index].wait_factor, index))
        .min()
        .unwrap();
    (
        Fitness {
            transfers,
            wait_factor: max(wait_factor_from_chains, wait_factor_from_queue),
        },
        queue,
    )
}

fn schedule_node<'a>(
    ready_nodes: &mut Vec<&'a ResolvedNode>,
    unscheduled_nodes: &mut Vec<usize>,
    nodes: &'a ResolvedNodeSet,
    node: &ResolvedNode,
    queue: usize,
    wait_factor: usize,
    submitted: usize,
    schedule: &mut Vec<QueueData>,
    images: &mut Vec<ChainData<Image>>,
    buffers: &mut Vec<ChainData<Buffer>>,
) {
    let queue_data = &mut schedule[queue];
    queue_data.wait_factor = max(queue_data.wait_factor, wait_factor + 1);
    let sid = queue_data
        .queue
        .add_submission(node.id, wait_factor, submitted, Unsynchronized);
    let submission = queue_data.queue.submission_mut(sid).unwrap();

    for &(id, state) in &node.buffers {
        add_to_chain(
            nodes.buffers[id],
            node.family,
            &mut buffers[id],
            sid,
            submission,
            state,
            |s, i, l| s.set_buffer_link(i, l),
        );
    }
    for &(id, state) in &node.images {
        add_to_chain(
            nodes.images[id],
            node.family,
            &mut images[id],
            sid,
            submission,
            state,
            |s, i, l| s.set_image_link(i, l),
        );
    }

    for &rev_dep in &node.rev_deps {
        unscheduled_nodes[rev_dep] -= 1;
        if unscheduled_nodes[rev_dep] == 0 {
            ready_nodes.push(&nodes.nodes[rev_dep]);
        }
    }
}

fn add_to_chain<R, S>(
    id: Id,
    family: rendy_core::hal::queue::QueueFamilyId,
    chain_data: &mut ChainData<R>,
    sid: SubmissionId,
    submission: &mut Submission<S>,
    state: State<R>,
    set_link: impl FnOnce(&mut Submission<S>, Id, usize),
) where
    R: Resource,
{
    let node = LinkNode { sid, state };

    chain_data.current_family = Some(family);
    chain_data.current_link_wait_factor = max(
        submission.wait_factor() + 1,
        chain_data.current_link_wait_factor,
    );

    let chain = &mut chain_data.chain;
    let chain_len = chain.links().len();
    let append = match chain.last_link_mut() {
        Some(ref mut link) if link.compatible(&node) => {
            set_link(submission, id, chain_len - 1);
            link.add_node(node);
            None
        }
        Some(_) | None => {
            set_link(submission, id, chain_len);
            chain_data.last_link_wait_factor = chain_data.current_link_wait_factor;
            Some(Link::new(node))
        }
    };

    if let Some(link) = append {
        chain.add_link(link);
    }
}
