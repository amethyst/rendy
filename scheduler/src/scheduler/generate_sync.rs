//! Given N schedules on different queues, will synthesize a minimal
//! synchronization strategy.
//!
//! As the algorithm runs, is maintains the following state:
//! * Current position in every resources dependency chain
//! * Current position in every queue
//!
//! The algorithm will simultaneously walk every dependency chain

use cranelift_entity::{SecondaryMap, entity_impl};
use cranelift_entity_set::EntitySet;

use rendy_core::hal;

use bumpalo::{
    Bump,
    collections::Vec as BVec,
};

use log::trace;

use super::{
    OrderIndependentSchedule, Scheduler, Entity, Resource,
    ScheduleEntry, SchedulerInput,
};
use crate::{
    resources::{ImageInfo, BufferInfo},
    builder::{ResourceKind, ImageKind, BufferKind},
};

#[derive(Clone)]
enum ResourceInfo {
    None,
    Image(ImageInfo),
    Buffer(BufferInfo),
}
impl Default for ResourceInfo {
    fn default() -> Self {
        ResourceInfo::None
    }
}

#[derive(Debug, Copy, Clone)]
enum FromSync {
    /// Sync based on the start of the current schedule.
    /// This can either involve a semaphore or not.
    Start,

    /// Syncs from another entity.
    Entity(Entity),
}

#[derive(Debug, Copy, Clone)]
struct AbstrSync {
    resource: Resource,
    from: FromSync,
    to: Entity,
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct Sync(u32);
entity_impl!(Sync, "sync");

struct SyncStrategy {
    //syncs: PrimaryMap<
}

impl Scheduler {

    pub(super) fn generate_sync(
        &mut self,
        builder: &SchedulerInput<(), ()>,
        unorder: &OrderIndependentSchedule,
        bump: &Bump,
    ) {
        let abstr_syncs = self.generate_required_syncs(builder, unorder, bump);

        println!("Abstr syncs: {:?}", abstr_syncs);
    }

    fn generate_required_syncs<'bump>(
        &mut self,
        builder: &SchedulerInput<(), ()>,
        unorder: &OrderIndependentSchedule,
        bump: &'bump Bump,
    ) -> BVec<'bump, AbstrSync>
    {

        let mut syncs = BVec::new_in(bump);

        let mut resource_cursors: SecondaryMap<Resource, (usize, usize)> = SecondaryMap::with_default((0, 0));

        // Scheduler only supports one queue right now, but the algorithm scales
        // trivially to any number.
        let mut queue_cursor = 0;
        let queue = &self.scheduled_order;

        // Preinitialize all resources with their initial state.
        let mut resource_states: SecondaryMap<Resource, FromSync> = SecondaryMap::with_default(FromSync::Start);
        //for resource in self.resource_schedule.resources() {
        //    resource_states[resource] = FromSync::Start;

        //    //match &builder.resource[resource] {
        //    //    ResourceKind::Image(data) => {
        //    //        match &data.kind {
        //    //            ImageKind::Provided { acquire, .. } => {
        //    //                resource_states[resource] = if acquire.is_some() {
        //    //                    Some(FromSync::Semaphore)
        //    //                } else {
        //    //                    Some(FromSync::Start)
        //    //                };
        //    //            },
        //    //            _ => (),
        //    //        }
        //    //    },
        //    //    ResourceKind::Buffer(data) => {
        //    //        match &data.kind {
        //    //            BufferKind::Provided { acquire, .. } => {
        //    //                resource_states[resource] = if acquire.is_some() {
        //    //                    Some(FromSync::Semaphore)
        //    //                } else {
        //    //                    Some(FromSync::Start)
        //    //                };
        //    //            },
        //    //            _ => (),
        //    //        }
        //    //    },
        //    //    _ => (),
        //    //}
        //}

        // The set of entities that have been resolved.
        let mut resolved: EntitySet<Entity> = EntitySet::new();

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
                ScheduleEntry::PassEntity(ent, pass) => {
                    let _pass = &self.passes[pass];
                    entity = ent;
                },
            }

            trace!("Entity: {}", entity);

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

                syncs.push(AbstrSync {
                    resource,
                    from: resource_states[resource],
                    to: entity,
                });
                resource_states[resource] = FromSync::Entity(entity);

                if curr_use.kind.is_write() {
                    let write = curr_use.kind.write();

                    assert!(curr_subuse_num == 0);
                    resource_cursors[resource] = (curr_use_idx + 1, 0);
                } else {
                    let write = curr_use.kind.read();

                    // Validate that the current entity actually is a read subuse
                    debug_assert!(write.iter().any(|u| u.entity == entity));

                    if write.len() == curr_subuse_num + 1 {
                        // If we have visited all of the reads for this subuse,
                        // bump to next use.
                        resource_cursors[resource] = (curr_use_idx + 1, 0);
                    } else {
                        resource_cursors[resource] = (curr_use_idx, curr_subuse_num + 1);
                    }
                }

            }

        }

        syncs
    }

}
