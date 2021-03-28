//! Given N schedules on different queues, will synthesize a minimal
//! synchronization strategy.
//!
//! As the algorithm runs, is maintains the following state:
//! * Current position in every resources dependency chain
//! * Current position in every queue
//!
//! The algorithm will simultaneously walk every dependency chain

use std::ops::Range;
use std::collections::BTreeMap;

use cranelift_entity::{PrimaryMap, SecondaryMap, ListPool, entity_impl, EntityList, EntityRef};
use cranelift_entity_set::EntitySet;

use rendy_core::hal;

use bumpalo::{
    Bump,
    collections::Vec as BVec,
};

use log::trace;

use super::{
    OrderIndependentSchedule, Scheduler, EntityId, ResourceId,
    ScheduleEntry, SchedulerInput, RenderPass,
};
use crate::{
    resources::{ImageInfo, BufferInfo},
    input::{SpecificResourceUseData, SyncPointKind},
    interface::{FenceId, SemaphoreId},
    sync::SyncPoint,
};

#[derive(Debug, Clone)]
pub struct LocalAbstr {
    pub resource: ResourceId,
    pub entities: Range<Option<EntityId>>,
    /// The indices in the scheduled order this sync is between.
    pub sync_indices: Range<usize>,
}
impl LocalAbstr {

    pub fn beyond_start(&self) -> bool {
        self.entities.start.is_none()
    }

    pub fn beyond_end(&self) -> bool {
        self.entities.end.is_none()
    }

    pub fn is_split(&self) -> bool {
        self.sync_indices.start != self.sync_indices.end
    }

    pub fn starts_at(&self, slot_idx: usize) -> bool {
        self.sync_indices.start == slot_idx
    }

}

#[derive(Debug, Clone)]
pub enum BarrierKind {
    Execution,
    Buffer {
        states: Range<hal::buffer::State>,
        target: ResourceId,
        range: hal::buffer::SubRange,
        families: Option<Range<hal::queue::family::QueueFamilyId>>,
    },
    Image {
        states: Range<hal::image::State>,
        target: ResourceId,
        range: hal::image::SubresourceRange,
        families: Option<Range<hal::queue::family::QueueFamilyId>>,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum BarrierOp {
    /// A full, normal barrier.
    Barrier,

    /// First half of split barrier, set event.
    SetEvent,
    /// Second half of split barrier, wait event.
    WaitEvent,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BarrierId(u32);
entity_impl!(BarrierId);

#[derive(Debug, Clone)]
pub struct BarrierData {
    /// Unique ID per barrier in generated sync.
    /// For normal barriers, this is unique.
    /// For splut barriers, this is the same for set and wait.
    pub id: BarrierId,

    /// The two entities this barrier applies between.
    pub entities: Range<Option<EntityId>>,
    /// Which mask of pipeline stages the dependency is between.
    pub stages: Range<hal::pso::PipelineStage>,

    /// What resource the barrier applies to.
    pub kind: BarrierKind,
    /// What kind of barrier this is.
    /// It can either be a full barrier, or one element of a split
    /// barrier.
    pub op: BarrierOp,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalAbstrId(u32);
entity_impl!(LocalAbstrId);

pub enum ExternalWait {
    Semaphore {

    },
}

#[derive(Debug, Copy, Clone)]
pub enum ExternalSignal {
    Semaphore(SemaphoreId),
    Fence(FenceId),
}

pub struct SyncSlot {
    pub local_abstr: EntityList<LocalAbstrId>,
    /// The range in the `barriers` array of `SyncStrategy` represented by
    /// this sync slot.
    pub barrier_range: Option<Range<usize>>,

    pub external_waits: Vec<ExternalWait>,
    pub external_signal: Vec<ExternalSignal>,
}

pub struct SyncStrategy {
    /// Vector with N+1 entries, where N is the number of entities in the
    /// graph.
    ///
    /// The slots are interspersed between the entities, as follows:
    /// <sync slot #0> <entity #0> <sync slot #1> <entity #1> <sync slot #2>
    pub slots: Vec<SyncSlot>,
    pub entity_to_slot_map: SecondaryMap<EntityId, Option<usize>>,

    pub last_usages: SecondaryMap<ResourceId, Option<EntityId>>,

    pub local_abstrs: PrimaryMap<LocalAbstrId, LocalAbstr>,
    pub local_abstr_pool: ListPool<LocalAbstrId>,

    pub barriers: Vec<BarrierData>,
    pub barrier_ids: PrimaryMap<BarrierId, ()>,
}

impl Default for SyncStrategy {
    fn default() -> Self {
        SyncStrategy {
            slots: Vec::new(),
            entity_to_slot_map: SecondaryMap::new(),
            last_usages: SecondaryMap::new(),
            local_abstrs: PrimaryMap::new(),
            local_abstr_pool: ListPool::new(),
            barriers: Vec::new(),
            barrier_ids: PrimaryMap::new(),
        }
    }
}

impl SyncStrategy {
    pub fn clear(&mut self) {
        self.slots.clear();
        self.entity_to_slot_map.clear();
        self.last_usages.clear();
        self.local_abstrs.clear();
        self.local_abstr_pool.clear();
        self.barriers.clear();
        self.barrier_ids.clear();
    }
}

impl Scheduler {

    pub(super) fn generate_sync<I: SchedulerInput>(
        &mut self,
        input: &I,
        unorder: &OrderIndependentSchedule,
        bump: &Bump,
    ) {
        self.sync_strategy.clear();

        for _n in 0..(self.scheduled_order.len() + 1) {
            self.sync_strategy.slots.push(SyncSlot {
                local_abstr: EntityList::new(),
                barrier_range: None,

                external_waits: Vec::new(),
                external_signal: Vec::new(),
            });
        }

        // Generate a simple abstract synchronization strategy.
        //
        // This simply consists of:
        // * A pair of entities, from and to
        // * The resource this applies to
        self.generate_required_syncs(input, unorder, bump);

        // Populate actual barriers with metadata
        self.generate_barriers(input);

        // Populate incoming synchronization (semaphores)
        self.generate_external_incoming(input);

        // Populate outgoing synchronization (signal semaphores/fences)
        self.generate_external_outgoing(input);

        self.debug_print_sync();
    }

    pub fn debug_print_sync(&self) {
        println!("==== begin abstr sync printout ====");
        for (slot_idx, slot) in self.sync_strategy.slots.iter().enumerate() {
            let abstrs = slot.local_abstr.as_slice(&self.sync_strategy.local_abstr_pool);
            for abstr in abstrs.iter() {
                println!("slot #{}: {:?}", slot_idx, &self.sync_strategy.local_abstrs[*abstr]);
            }
        }
        println!("==== end abstr sync printout ====");

        println!("==== begin barriers printout ====");
        for barrier in self.sync_strategy.barriers.iter() {
            println!("{:?}", barrier);
        }
        println!("==== end barriers printout ====");

        println!("==== begin outgoing printout ====");
        for (slot_idx, slot) in self.sync_strategy.slots.iter().enumerate() {
            for signal in slot.external_signal.iter() {
                println!("slot #{}: {:?}", slot_idx, signal);
            }
        }
        println!("==== end outgoing printout ====");
    }

    fn generate_external_incoming<I: SchedulerInput>(
        &mut self,
        input: &I,
    ) {
    }

    fn generate_external_outgoing<I: SchedulerInput>(
        &mut self,
        input: &I,
    ) {
        let num_semaphores = input.num_semaphores();
        for semaphore_n in 0..num_semaphores {
            let semaphore_id = SemaphoreId::new(semaphore_n);
            let root_sync_point = input.get_semaphore(semaphore_id);

            fn foo<I: SchedulerInput>(sync_point: SyncPoint, input: &I, sync: &mut SyncStrategy) -> usize {
                let kind = input.get_sync_point(sync_point);
                match kind {
                    SyncPointKind::Resource(resource, use_idx) => {
                        // The point in the schedule is specified by a resource use idx.
                        // Get the ResourceUseId of that index.
                        let use_id = input.get_uses(resource)[use_idx];

                        // Get the entity referenced by that ResourceUseId
                        let entity = input.resource_use_data(use_id).entity;

                        // The entity has a slot in the sync list.
                        let slot_idx = sync.entity_to_slot_map[entity].unwrap() + 1;

                        slot_idx
                    },
                    SyncPointKind::And(s1, s2) => {
                        todo!()
                    },
                }
            }

            let slot_idx = foo(root_sync_point, input, &mut self.sync_strategy);
            self.sync_strategy.slots[slot_idx].external_signal.push(ExternalSignal::Semaphore(semaphore_id));
        }
    }

    fn generate_barriers<I: SchedulerInput>(
        &mut self,
        input: &I,
    ) {
        // Map of (resource, end sync slot) -> barrier id
        let mut barrier_map: BTreeMap<(ResourceId, usize), BarrierData> = BTreeMap::new();

        for (slot_idx, slot) in self.sync_strategy.slots.iter_mut().enumerate() {
            let abstr_ids = slot.local_abstr.as_slice(&self.sync_strategy.local_abstr_pool);

            let barriers_start_idx = self.sync_strategy.barriers.len();

            for abstr_id in abstr_ids.iter() {
                let abstr = &self.sync_strategy.local_abstrs[*abstr_id];

                if abstr.starts_at(slot_idx) {
                    // If the abstr sync starts at the current slot, we need to
                    // collect the barrier data. This also applies to regular
                    // barriers.

                    match abstr.entities {
                        Range { start: Some(start), end: Some(end) } => {
                            let start_aux = self.resource_schedule.aux(start, abstr.resource);
                            let start_use = input.resource_use_data(start_aux.use_id);

                            let end_aux = self.resource_schedule.aux(end, abstr.resource);
                            let end_use = input.resource_use_data(end_aux.use_id);

                            let kind = match (&start_use.specific_use_data, &end_use.specific_use_data) {
                                (
                                    SpecificResourceUseData::Buffer { state: start_state },
                                    SpecificResourceUseData::Buffer { state: end_state }
                                ) => {
                                    BarrierKind::Buffer {
                                        states: (*start_state)..(*end_state),
                                        target: abstr.resource,
                                        range: hal::buffer::SubRange::WHOLE,
                                        families: None,
                                    }
                                },
                                (
                                    SpecificResourceUseData::Image { state: start_state },
                                    SpecificResourceUseData::Image { state: end_state }
                                ) => {
                                    BarrierKind::Image {
                                        states: (*start_state)..(*end_state),
                                        target: abstr.resource,
                                        range: hal::image::SubresourceRange::default(),
                                        families: None,
                                    }
                                },
                                _ => unreachable!(),
                            };

                            let op = match (abstr.sync_indices.start, abstr.sync_indices.end) {
                                (l, r) if l == r => BarrierOp::Barrier,
                                (l, _r) if l == slot_idx => BarrierOp::SetEvent,
                                (_l, r) if r == slot_idx => BarrierOp::WaitEvent,
                                _ => unreachable!(),
                            };

                            let data = BarrierData {
                                id: self.sync_strategy.barrier_ids.push(()),
                                entities: abstr.entities.clone(),
                                stages: start_use.stages..end_use.stages,
                                kind,
                                op,
                            };

                            self.sync_strategy.barriers.push(data.clone());

                            barrier_map.insert((abstr.resource, abstr.sync_indices.end), data);
                        },
                        Range { start: None, end: Some(end) } => {
                            let end_aux = self.resource_schedule.aux(end, abstr.resource);
                            let end_use = input.resource_use_data(end_aux.use_id);

                        },
                        Range { start: Some(start), end: None } => {},
                        Range { start: None, end: None } => unreachable!(),
                    }

                } else {
                    // If the abstr sync ends at the current slot, we simply
                    // look up an already created barrier and insert the end
                    // marker. Regular barriers only have one abstr sync entry,
                    // so this codepath will never be triggered.

                    let mut data = barrier_map[&(abstr.resource, abstr.sync_indices.end)].clone();
                    data.op = BarrierOp::WaitEvent;
                    self.sync_strategy.barriers.push(data);
                }
            }

            let barriers_end_idx = self.sync_strategy.barriers.len();
            slot.barrier_range = Some(barriers_start_idx..barriers_end_idx);
        }


    }

    fn generate_required_syncs<'bump, I: SchedulerInput>(
        &mut self,
        input: &I,
        unorder: &OrderIndependentSchedule,
        bump: &'bump Bump,
    ) {

        let mut resource_cursors: SecondaryMap<ResourceId, (usize, usize)> = SecondaryMap::with_default((0, 0));

        // Scheduler only supports one queue right now, but the algorithm scales
        // trivially to any number.
        let mut queue_cursor = 0;
        let queue = &self.scheduled_order;

        // Preinitialize all resources with their initial state.
        //let mut resource_states: SecondaryMap<ResourceId, Option<EntityId>> = SecondaryMap::with_default(None);

        // The set of entities that have been resolved.
        let mut resolved: EntitySet<EntityId> = EntitySet::new();

        // Iterates over queue indices where the cursor is incremented.
        for curr_queue in self.schedule_traversal.iter().cloned() {

            // TODO: Change to multi-queue, lookup correct queue here.
            assert!(curr_queue == 0);
            let queue_idx = queue_cursor;
            queue_cursor += 1;

            let entity;
            match queue[queue_idx] {
                ScheduleEntry::General(ent) => {
                    entity = ent;
                },
                ScheduleEntry::PassEntity(ent, pass, _subpass_idx) => {
                    let _pass = &self.passes[pass];
                    entity = ent;
                },
            }

            trace!("Entity: {}", entity);

            self.sync_strategy.entity_to_slot_map[entity] = Some(queue_idx);

            // Sanity check for schedule traversal.
            // Validate that the current entity actually had all its
            // dependencies satisfied.
            #[cfg(debug_assertions)]
            {
                resolved.insert(entity, &mut self.entity_set_pool);
                assert!(self.rev_cum_deps[entity].difference(&resolved, &self.entity_set_pool).count() == 0);
            }

            for (resource, aux) in self.resource_schedule.usages_by(entity) {
                let (curr_use_idx, curr_subuse_num) = resource_cursors[resource];
                let dat = &unorder.resources[resource];

                println!("{}: {:?}", resource, dat);

                let curr_use = &dat.uses[curr_use_idx];

                let prev_entity = self.sync_strategy.last_usages[resource];

                let prev_sync_idx = prev_entity
                    .map(|e| self.sync_strategy.entity_to_slot_map[e].unwrap() + 1)
                    .unwrap_or(0);

                let abstr_id = self.sync_strategy.local_abstrs.push(LocalAbstr {
                    resource,
                    entities: prev_entity..Some(entity),
                    sync_indices: prev_sync_idx..queue_idx,
                });
                self.sync_strategy.last_usages[resource] = Some(entity);

                self.sync_strategy.slots[queue_idx].local_abstr.push(abstr_id, &mut self.sync_strategy.local_abstr_pool);

                if prev_sync_idx != queue_idx {
                    self.sync_strategy.slots[prev_sync_idx].local_abstr.push(abstr_id, &mut self.sync_strategy.local_abstr_pool);
                }

                if curr_use.kind.is_write() {
                    let _write = curr_use.kind.write();

                    assert!(curr_subuse_num == 0);
                    resource_cursors[resource] = (curr_use_idx + 1, 0);
                } else {
                    let read = curr_use.kind.read();

                    // Validate that the current entity actually is a read subuse
                    debug_assert!(read.iter().any(|u| u.entity == entity));

                    if read.len() == curr_subuse_num + 1 {
                        // If we have visited all of the reads for this subuse,
                        // bump to next use.
                        resource_cursors[resource] = (curr_use_idx + 1, 0);
                    } else {
                        resource_cursors[resource] = (curr_use_idx, curr_subuse_num + 1);
                    }
                }

            }

        }

        for (resource, state) in self.sync_strategy.last_usages.iter() {
            if let Some(entity_id) = state {

                let prev_sync_idx = self.sync_strategy.entity_to_slot_map[*entity_id].unwrap() + 1;
                let end_sync_idx = self.sync_strategy.slots.len() - 1;

                let abstr_id = self.sync_strategy.local_abstrs.push(LocalAbstr {
                    resource,
                    entities: Some(*entity_id)..None,
                    sync_indices: prev_sync_idx..end_sync_idx,
                });

                self.sync_strategy.slots[prev_sync_idx].local_abstr.push(abstr_id, &mut self.sync_strategy.local_abstr_pool);
                if prev_sync_idx != end_sync_idx {
                    self.sync_strategy.slots[end_sync_idx].local_abstr.push(abstr_id, &mut self.sync_strategy.local_abstr_pool);
                }
            }
        }
    }

}
