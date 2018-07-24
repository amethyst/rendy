//! Transient memory allocator.

extern crate permutohedron;

use std::{cmp::{max, Ordering}, collections::{HashMap, HashSet, BTreeMap, BTreeSet}, ops::Range};

/// Resource index.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceId(pub usize);

/// Task index.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub usize);

/// Resource info (size).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resource {
    pub size: u64
}

/// Task resources and task ids this tasks run after.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Task {
    resources: Vec<ResourceId>,

    // Tasks that completes before this one starts.
    before: BTreeSet<TaskId>,

    // Tasks that starts after this one completes.
    after: BTreeSet<TaskId>,
}

impl Task {
    pub fn new(resources: impl IntoIterator<Item = ResourceId>, dependencies: impl IntoIterator<Item = TaskId>) -> Self {
        Task {
            resources: resources.into_iter().collect(),
            before: dependencies.into_iter().collect(),
            after: BTreeSet::new(),
        }
    }
}

/// Job is a collection of tasks and resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    tasks: Vec<Task>,
    resources: Vec<Resource>,
    disjoints: Vec<HashSet<ResourceId>>,
    ids: Option<Vec<ResourceId>>,
}

impl Job {
    pub fn new(mut tasks: Vec<Task>, resources: Vec<Resource>) -> Self {
        complete_task_relations(&mut tasks);
        let disjoints = find_disjoints(&tasks);
        let job = Job {
            tasks,
            resources,
            disjoints,
            ids: None,
        };
        println!("{:#?}", job);
        job
    }

    pub fn is_disjoint(&self, left: ResourceId, right: ResourceId) -> bool {
        self.disjoints[left.0].contains(&right)
    }

    fn reset_variants(&mut self) {
        self.ids = None;
    }

    fn next_variant(&mut self) -> Option<(u64, HashMap<ResourceId, Range<u64>>)> {
        let ids: &[ResourceId] = match self.ids {
            Some(ref mut ids) => if !next_permutation(ids) {
                return None
            } else {
                ids
            },
            ref mut ids => {
                *ids = Some((0 .. self.resources.len()).map(ResourceId).collect());
                ids.as_ref().unwrap()
            }
        };

        // ordered allocations.
        let mut allocations: BTreeMap<u64, Vec<ResourceId>> = BTreeMap::new();

        let ref disjoints = self.disjoints;
        let ref resources = self.resources;

        for &id in ids {
            let disjoints = disjoints.get(id.0);
            let overlapping = allocations.iter().filter_map(|(&start, ids)| {
                if disjoints.map_or(false, |disjoints| ids.iter().all(|id| disjoints.contains(id))) {
                    None
                } else {
                    Some(start .. start + ids.iter().map(|id| resources[id.0].size).max().unwrap_or(0))
                }
            }).collect::<Vec<_>>();
            let mut end = 0;
            'a: for range in overlapping {
                if range.start > end && range.start - end > resources[id.0].size {
                    break 'a;
                }
                end = max(end, range.end);
            }
            allocations.entry(end).or_insert(Vec::new()).push(id);
        }

        let mut total = 0;
        let result = allocations.into_iter()
            .flat_map(|(start, ids)| {
                ids.into_iter().map(move |id| {
                    let end = start + resources[id.0].size;
                    (id, start .. end)
                })
            })
            .inspect(|(_, range)| total = max(total, range.end))
            .collect();

        Some((total, result))
    }

    pub fn verify(&self, allocations: impl IntoIterator<Item = (ResourceId, Range<u64>)>) {
        // TODO
    }

    pub fn variants<'a>(&'a mut self) -> impl Iterator<Item = (u64, HashMap<ResourceId, Range<u64>>)> + 'a {
        ::std::iter::repeat(()).scan((), move |&mut (), ()| {
            match self.next_variant() {
                Some((total, allocations)) => {
                    self.verify(allocations.iter().map(|(&id, range)| (id, range.clone())));
                    Some((total, allocations))
                }
                None => {
                    self.reset_variants();
                    None
                }
            }
        })
    }
}

fn complete_task_relations(tasks: &mut [Task]) {
    for index in 0 .. tasks.len() {
        let more = tasks[index].before.iter().flat_map(|before| {
            assert!(before.0 < index);
            tasks[before.0].before.clone()
        }).collect::<Vec<_>>();
        tasks[index].before.extend(more);
    }

    for index in 0 .. tasks.len() {
        let before = tasks[index].before.clone();
        for before in before {
            tasks[before.0].after.insert(TaskId(index));
        }
    }
}

fn task_order(tasks: &[Task], left: TaskId, right: TaskId) -> Option<Ordering> {
    if left == right {
        Some(Ordering::Equal)
    } else if tasks[right.0].after.contains(&left) {
        Some(Ordering::Less)
    } else if tasks[left.0].after.contains(&right) {
        Some(Ordering::Greater)
    } else {
        None
    }
}

/// Find if tasks are disjoint.
/// Either all tasks from left set complete before any of tasks from right set.
/// Or all tasks from right set complete before any of tasks from left set.
fn check_tasks_disjoint(tasks: &[Task], left: &[TaskId], right: &[TaskId]) -> bool {
    let ordering = match (left.iter().next(), right.iter().next()) {
        (None, _) | (_, None) => return true, // Empty set is disjoint with any set.
        (Some(&left), Some(&right)) => task_order(tasks, left, right),
    };

    match ordering {
        None | Some(Ordering::Equal) => return false, // If tasks are incomparable or equal then sets are not disjoint.
        _ => {}
    }

    for &left in left {
        for &right in right {
            if task_order(tasks, left, right) != ordering {
                // If any pair of tasks has different ordering than other pairs then sets are not disjoint.
                return false
            }
        }
    }

    // All tasks from left are before or all are after tasks from right.
    return true
}

/// Calculate disjoint sets for all resources.
fn find_disjoints(tasks: &[Task]) -> Vec<HashSet<ResourceId>> {
    let mut task_sets: HashMap<ResourceId, BTreeSet<TaskId>> = HashMap::new();
    let mut resource_count = 0;

    // Collect task sets for all resources.
    for (index, task) in tasks.iter().enumerate() {
        for &resource in &task.resources {
            resource_count = max(resource_count, resource.0 + 1);
            task_sets.entry(resource).or_insert_with(|| BTreeSet::new()).insert(TaskId(index));
        }
    }

    println!("task_sets: {:#?}", task_sets);

    // Reduce task sets removing tasks that are between others.
    // E.g. there is task in set that is before and another one that is after.
    let resource_tasks = task_sets.iter().map(|(resource, set)| {
        let tasks = set.iter().cloned().filter(|task| {
            tasks[task.0].before.is_disjoint(&*set) || tasks[task.0].after.is_disjoint(&*set)
        }).collect::<Vec<TaskId>>();

        (resource.0, tasks)
    }).collect::<Vec<_>>();

    println!("resource_tasks: {:#?}", resource_tasks);

    let mut disjoints: Vec<HashSet<ResourceId>> = vec![HashSet::new(); resource_count];

    for (index, left) in resource_tasks.iter().enumerate() {
        for right in resource_tasks.iter().skip(index+1) {
            if check_tasks_disjoint(tasks, &left.1, &right.1) {
                disjoints[left.0].insert(ResourceId(right.0));
                disjoints[right.0].insert(ResourceId(left.0));
            }
        }
    }

    println!("disjoints: {:#?}", disjoints);

    disjoints
}

fn next_permutation(values: &mut [ResourceId]) -> bool {
    permutohedron::LexicalPermutation::next_permutation(values)
}


#[cfg(test)]
mod test {
    extern crate rand;
    use self::rand::*;
    use self::rand::distributions::Distribution;
    use super::*;

    const RESOURCE_SIZE_LIMIT: u64 = 65536;
    const RESOURCE_COUNT: usize = 128;
    const TASK_COUNT: usize = 32;

    #[test]
    fn fuzz() {
        let mut rng = prng::ChaChaRng::from_entropy();
        let random_resource_size = distributions::Uniform::from(0 .. RESOURCE_SIZE_LIMIT);
        let random_resource_count = distributions::Uniform::from(0 .. RESOURCE_COUNT);
        let random_task_count = distributions::Uniform::from(0 .. TASK_COUNT);

        let resources = (0 .. RESOURCE_COUNT).map(|_| Resource { size: random_resource_size.sample(&mut rng) }).collect::<Vec<_>>();
        let mut tasks = (0 .. TASK_COUNT).map(|index| {
            let count = if index > 0 { (random_task_count.sample(&mut rng) % index) } else { 0 };
            (seq::sample_indices(&mut rng, index, count).into_iter().map(TaskId).collect::<Vec<_>>(), Vec::new())
        }).collect::<Vec<_>>();

        for id in 0 .. RESOURCE_COUNT {
            let count = random_task_count.sample(&mut rng) / 8 + 1;
            for index in seq::sample_indices(&mut rng, TASK_COUNT, count) {
                tasks[index].1.push(ResourceId(id));
            }
        }

        let tasks = tasks.into_iter().map(|(dependencies, resources)| Task::new(resources, dependencies)).collect::<Vec<_>>();

        let mut job = Job::new(tasks, resources);

        let (total, allocations) = job
            .variants()
            .take(1024)
            .min_by_key(|&(total, ref allocations)| {
                assert_eq!(allocations.len(), RESOURCE_COUNT);
                total
            }).unwrap();

        let required = allocations.iter().fold(0, |acc, (_, range)| {
            acc + (range.end - range.start)
        });
        
        println!("
total: {},
required: {},
saved: {}
allocations {:#?}
        ", total, required, required - total, allocations);
    }
}