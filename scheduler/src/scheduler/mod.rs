use std::marker::PhantomData;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;

use log::{trace};

use cranelift_entity::{PrimaryMap, SecondaryMap, ListPool, EntityList, EntityRef, entity_impl};
use cranelift_entity_set::{EntitySet, EntitySetPool};

use bumpalo::Bump;

use rendy_core::hal;

use crate::{BufferId, ImageId};
//use super::{
//    IterEither,
//    //node::EntityId,
//    //idx_vec::{IdxImpl, IdxVec},
//    //builder::procedural::{ProceduralBuilder, ResourceId, EntityKind, ResourceKind, ImageUsageKind},
//};

mod resource_schedule;
use resource_schedule::{NaturalScheduleMatrix, Direction, NaturalIndexMapping};

//mod pooled_linked_list;
//use pooled_linked_list::{LinkedListPool, List};

mod identify_render_passes;

mod generate_sync;

mod order_independent_schedule;
use order_independent_schedule::OrderIndependentSchedule;

pub mod input;
pub use input::{
    Entity, Resource, UseKind, EntityKind, SchedulerInput, RenderPassSpan,
    EntityData, ResourceData,
};

//fn propagate<I: Copy + Eq + Ord>(map: &mut BTreeMap<I, I>) {
//    let keys: Vec<_> = map.keys().cloned().collect();
//    loop {
//        let mut changed = false;
//
//        for key in keys.iter() {
//            let to_1 = map[&key];
//            if let Some(to_2) = map.get(&to_1).cloned() {
//                map.insert(*key, to_2);
//            }
//        }
//
//        if !changed { break; }
//    }
//}
//
//fn resolve_aliases<I: EntityRef + Ord, T, F>(vec: &PrimaryMap<I, T>, resolved: &mut BTreeMap<I, I>, fun: F)
//where
//    F: Fn(&T) -> Option<I>,
//{
//    debug_assert!(resolved.len() == 0);
//
//    for (id, item) in vec.iter() {
//        if let Some(alias) = fun(item) {
//            resolved.insert(id, alias);
//        }
//    }
//
//    propagate(resolved);
//}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RenderPass(u32);
entity_impl!(RenderPass);

/// One scheduling strategy might not be optimal for everything, this provides
/// the option of tuning the strategy. Allows you to tune things like
/// aggressiveness when merging subpasses.
#[derive(Debug, Clone)]
pub struct SchedulerStrategy {
    pub subpass_strategy: SubpassStrategy,
}

#[derive(Debug, Copy, Clone)]
pub enum SubpassStrategy {
    /// This will schedule everything in its own render pass, even if things are
    /// manually annotated.
    None,

    /// This will simply validate that the manually annotated entities are
    /// schedulable in the same pass, and do that.
    Annotated,

    /// This will do everything `Annotated` does, bug will also try to combine
    /// things on its own.
    Aggressive,
}

#[derive(Debug)]
pub(crate) enum ScheduleEntry {
    General(Entity),
    PassEntity(Entity, RenderPass),
}

struct RenderPassData {
    entities: EntityList<Entity>,
    members: EntitySet<Entity>,

    attachments: EntitySet<Resource>,
    uses: EntitySet<Resource>,
    writes: EntitySet<Resource>,

    for_cum: EntitySet<Entity>,
    rev_cum: EntitySet<Entity>,
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum UsageKind {
    Attachment,
    InputAttachment,
    Descriptor,
    ReadDescriptor,
}
impl UsageKind {
    fn is_attachment(&self) -> bool {
        match self {
            UsageKind::Attachment => true,
            UsageKind::InputAttachment => true,
            UsageKind::Descriptor => false,
            UsageKind::ReadDescriptor => false,
        }
    }
    fn is_write(&self) -> bool {
        match self {
            UsageKind::Attachment => true,
            UsageKind::InputAttachment => false,
            UsageKind::Descriptor => true,
            UsageKind::ReadDescriptor => false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct ScheduleAux {
    usage_num: u16,
    usage_kind: UsageKind,
}

/// Scheduler, takes a `ProceduralBuilder` containing the built render graph and
/// determines order and synchronization.
///
pub struct Scheduler {
    //resource_aliases: BTreeMap<Resource, Resource>,
    resource_schedule: NaturalScheduleMatrix<Entity, Resource, ScheduleAux>,

    // For every entity in the graph, this contains the set of other entities
    // that are strictly required to be scheduled before or after respectively,
    // due to dependencies.
    //
    // Crucually, these don't contain "false" dependencies, read dependencies on
    // read uses. This makes querying for entities that need to be scheduled
    // between two others a simple set intersection.
    for_cum_deps: SecondaryMap<Entity, EntitySet<Entity>>,
    rev_cum_deps: SecondaryMap<Entity, EntitySet<Entity>>,

    // Stage 1: Render pass grouping
    active_passes: BTreeSet<RenderPass>,
    passes: PrimaryMap<RenderPass, RenderPassData>,
    passes_back: SecondaryMap<Entity, Option<RenderPass>>,

    // Stage 3
    pub(crate) scheduled_order: Vec<ScheduleEntry>,
    schedule_traversal: Vec<usize>,

    // Pools
    entity_list_pool: ListPool<Entity>,
    entity_set_pool: EntitySetPool<Entity>,
    resource_set_pool: EntitySetPool<Resource>,

    bump: Option<Bump>,

}

impl Scheduler {

    pub fn new() -> Self {
        Self {
            //resource_aliases: BTreeMap::new(),
            resource_schedule: NaturalScheduleMatrix::new(NaturalIndexMapping::new(), NaturalIndexMapping::new()),

            for_cum_deps: SecondaryMap::new(),
            rev_cum_deps: SecondaryMap::new(),

            active_passes: BTreeSet::new(),
            passes: PrimaryMap::new(),
            passes_back: SecondaryMap::with_default(None),

            scheduled_order: Vec::new(),
            schedule_traversal: Vec::new(),

            entity_list_pool: ListPool::new(),
            entity_set_pool: EntitySetPool::new(),
            resource_set_pool: EntitySetPool::new(),

            bump: Some(Bump::new()),
        }
    }

    pub fn plan(&mut self/*, strategy: &SchedulerStrategy*/, builder: &input::SchedulerInput<(), ()>) {
        trace!("==== Scheduler plan start");
        let bump = self.bump.take().unwrap();

        {

            // As a concequence of the move mechanism, images and buffers may be
            // aliases. This resolves chains of aliases into a map we can use for
            // the rest of the planning phase.
            //trace!("==== == resolve_aliases");
            //resolve_aliases(&builder.resources, &mut self.resource_aliases, |d| d.is_alias());

            // Process all resource uses into a resource schedule, a special data
            // structure used to enable efficient walking up and down the dependency
            // chains of resources.
            trace!("==== == populate_base_schedule");
            self.populate_base_schedule(builder);

            let unorder_schedule = OrderIndependentSchedule::new(&self.resource_schedule, &bump);

            // Contains the dependencies and dependents in a cumulative manner.
            trace!("==== == populate_cumulative_deps");
            self.populate_cumulative_deps();

            // ==== Step 1: Identify render passes
            trace!("==== == identify_render_passes");
            self.identify_render_passes(builder);

            // ==== Step 2: Allocate queues
            // TODO: Right now everything is scheduled on one single queue.
            // It is at this step we would want to decide what chains in the graph
            // are disjoint enough to get effectively scheduled on another queue.
            // All scheduling steps after this and before synchronization generation
            // would then happen once per queue.
            //
            // UPDATE2: After reading some more I suppose automatic moving of stuff
            // to other queues might not be such a good idea. I guess we want to
            // support manually annotating this, but I suppose the main application
            // area for this would be async compute. Async compute would probably
            // be better supported through the mechanism of submitting multiple
            // graphs?

            // ==== Step 3: Optimize ordering
            // TODO: Right now things are scheduled at declaration order from the
            // builder (with the exception of reorderings done due to render pass
            // grouping). Entities can be reordered as long as order is preserved
            // for all resource use chains, and should be reordered to maximize
            // interspersion between dependency chains (possibly with weights per
            // entity).
            // This is pretty much a variation of a classic instruction scheduling
            // problem, and can probably be handled pretty efficiently with
            // algorithms from that domain.
            //
            // UPDATE2: After some thinking I have an algorithm in mind that should
            // accomplish the following requirements:
            // * Satisfy dependencies, obviously.
            // * Intersperse chains based on cost.
            // * It runs in O(N)-ish where N is somewhere between the complexity of
            // the graph and the hypergraph. (I think, maybe not, but it shouldn't
            // be bad, and is really deterministic) Primary goal here is to avoid
            // a iterative optimization algorithm.
            // * It finds a globally optimal-ish solution, unlike iterative
            // optimization which I can imagine would have local minimums.
            //
            // Algorithm:
            // 1. Assign a cost to every node. We can probably make up some pretty
            // good heuristics for this and allow for customization by the user.
            // 2. (MAYBE, if cost becomes a problem? Probably not) Reduce the graph
            // to a "hypergraph" where the topology of the graph is preserved, and
            // costs are added together. This means that for a dependency chain of
            // several nodes, they are merged into one node. The cost of traversing
            // the hypergraph will be the same as traversing the original graph,
            // except traversal of fewer nodes are needed. This is done to reduce
            // the complexity of our balancing algorithm since it fundimentally
            // operates on a graph topology level.
            // 3. For each root, find the longest (critical) path in the DAG. We
            // pick the path with the highest total cost as the starting point.
            // 4. For each root that hasn't yet been processed, starting with
            // longest critical path: Find shortest path to any processed node
            // (start of critical path if unprocessed), and normalize the cost from
            // that node to the root, starting with the end timeline point of the
            // closest node.
            // 5. Sort the nodes by the average between the start and end timeline
            // point. This is your scheduling order.
            trace!("==== == generate_naive_order");
            self.generate_order_naive(builder);

            trace!("Scheduled order: {:?}", self.scheduled_order);

            // ==== Step 4: Generate synchronization
            // At this point we should have a good ordering we want to generate
            // synchronization for.
            trace!("==== == generate_synchronization");
            self.generate_sync(builder, &unorder_schedule, &bump);

        }

        self.bump = Some(bump);
        trace!("==== Scheduler plan end");
    }

    fn populate_base_schedule(&mut self, builder: &input::SchedulerInput<(), ()>) {
        self.resource_schedule.clear();
        self.resource_schedule.set_dims(
            builder.entity.len(),
            builder.resource.len(),
        );
        self.resource_schedule.populate(|res_id| {
            let res = &builder.resource[res_id];
            Some(
                res.uses
                   .iter(&builder.resource_use_set_pool)
                   .enumerate()
                   .map(|(idx, res_use)| {
                       let use_data = &builder.resource_use[res_use];
                       let usage_kind = match (use_data.is_write, use_data.use_kind) {
                           (true, UseKind::Use) => UsageKind::Descriptor,
                           (false, UseKind::Use) => UsageKind::ReadDescriptor,
                           (true, UseKind::Attachment) => UsageKind::Attachment,
                           (false, UseKind::Attachment) => UsageKind::InputAttachment,
                       };
                       (
                           use_data.entity,
                           ScheduleAux {
                               usage_num: idx.try_into().unwrap(),
                               usage_kind,
                           },
                       )
                   })
            )
            //match res {
            //    ResourceKind::Image(img) => {
            //        Some(IterEither::A(
            //            img.uses.iter().enumerate().map(|(n, u)| (
            //                u.by,
            //                ScheduleAux {
            //                    usage_num: n.try_into().unwrap(),
            //                    usage_kind: match (u.usage.is_write(), &u.kind) {
            //                        (true, ImageUsageKind::Use) => UsageKind::Descriptor,
            //                        (false, ImageUsageKind::Use) => UsageKind::ReadDescriptor,
            //                        (true, ImageUsageKind::Attachment) => UsageKind::Attachment,
            //                        (false, ImageUsageKind::Attachment) => UsageKind::Attachment,
            //                        (true, ImageUsageKind::InputAttachment) => panic!(),
            //                        (false, ImageUsageKind::InputAttachment) => UsageKind::InputAttachment,
            //                    },
            //                },
            //            ))
            //        ))
            //    },
            //    ResourceKind::Buffer(buf) => {
            //        Some(IterEither::B(
            //            buf.uses.iter().enumerate().map(|(n, u)| (
            //                u.by,
            //                ScheduleAux {
            //                    usage_num: n.try_into().unwrap(),
            //                    usage_kind: match u.usage.is_write() {
            //                        true => UsageKind::Descriptor,
            //                        false => UsageKind::ReadDescriptor,
            //                    },
            //                }
            //            ))
            //        ))
            //    },
            //    ResourceKind::Alias(_) => None,
            //}
        });
    }

    fn populate_cumulative_deps(&mut self) {

        let resource_schedule = &self.resource_schedule;
        let pool = &mut self.entity_set_pool;

        // As we go down the matrix, this contains the last write usage of the resource.
        // This is different than the last use, as that will include reads.
        // The fact that our cumulative dependency sets don't contain reads is what makes
        // them useful for quickly checking validity of merges/reorders.
        let mut prev_write: SecondaryMap<Resource, Option<Entity>> = SecondaryMap::with_default(None);

        let mut do_pass = |cum: &mut SecondaryMap<Entity, EntitySet<Entity>>, dir: Direction| {
            for ent in resource_schedule.entities(dir) {
                println!("{}", ent);

                // The collected deps for this pass
                let mut collected = EntitySet::new();

                for (res_id, aux) in resource_schedule.usages_by(ent) {

                    println!("== {}", res_id);

                    // If we have a previous write usage of the resource, merge the
                    // dep set of that entity into the current set.
                    println!("==== prev write {:?}", prev_write[res_id]);
                    if let Some(prev_write_ent) = prev_write[res_id] {
                        collected.union_into(&cum[prev_write_ent], pool);
                        //collected.insert(prev_write_ent, pool);
                    }

                    // If the current resource use is a write, we update the
                    // `prev_write` for the resource, and merge all the usages
                    // in between to the current set.
                    if aux.usage_kind.is_write() {
                        if let Some(prev_write_ent) = prev_write[res_id] {
                            let mut iter = resource_schedule.usages_between(
                                prev_write_ent, ent, dir, res_id);
                            iter.next();
                            for (entity, _aux) in iter {
                                collected.union_into(&cum[entity], pool);
                                //collected.insert(entity, pool);
                            }
                        }

                        prev_write[res_id] = Some(ent);
                    }

                }

                let mut copy = collected.make_copy(pool);
                copy.insert(ent, pool);

                cum[ent] = copy;

            }

            for ent in resource_schedule.entities(Direction::Forward) {
                println!("{}: {:?}", ent, cum[ent].bind(pool));
            }

            prev_write.clear();
        };

        // Forward pass
        trace!("FORWARD PASS");
        do_pass(&mut self.rev_cum_deps, Direction::Forward);

        // Reverse pass
        trace!("REVERSE PASS");
        do_pass(&mut self.for_cum_deps, Direction::Reverse);

    }

    /// A simple and naive greedy scheduling strategy.
    /// This will simply pick the first schedulable entity at each iteration
    /// until all entities are scheduled.
    fn generate_order_naive(&mut self, builder: &input::SchedulerInput<(), ()>) {

        fn try_schedule_entity(
            entity: Entity,
            scheduled_mask: &mut EntitySet<Entity>,
            overdue: &mut EntitySet<Entity>,
            scheduled_order: &mut Vec<ScheduleEntry>,
            rev_cum_deps: &SecondaryMap<Entity, EntitySet<Entity>>,
            passes: &PrimaryMap<RenderPass, RenderPassData>,
            passes_back: &SecondaryMap<Entity, Option<RenderPass>>,
            entity_set_pool: &mut EntitySetPool<Entity>,
            entity_list_pool: &ListPool<Entity>,
        ) -> bool
        {
            if let Some(pass_id) = passes_back[entity] {
                let pass = &passes[pass_id];

                if pass.rev_cum
                       .difference(&scheduled_mask, entity_set_pool)
                       .filter(|v| !pass.members.contains(*v, entity_set_pool))
                       .count() > 0
                {
                    return false;
                }


                for entity in pass.entities.as_slice(&entity_list_pool) {
                    scheduled_order.push(ScheduleEntry::PassEntity(*entity, pass_id));

                    scheduled_mask.insert(*entity, entity_set_pool);
                    overdue.remove(*entity, entity_set_pool);
                }

            } else {

                if rev_cum_deps[entity]
                    .difference(&scheduled_mask, entity_set_pool)
                    .filter(|v| *v != entity)
                    .count() > 0
                {
                    return false;
                }

                scheduled_order.push(ScheduleEntry::General(entity));
                scheduled_mask.insert(entity, entity_set_pool);
                overdue.remove(entity, entity_set_pool);

            }

            true
        }

        let mut scheduled_mask = EntitySet::new();
        let mut overdue = EntitySet::new();

        for entity in self.resource_schedule.entities(Direction::Forward) {

            if scheduled_mask.contains(entity, &self.entity_set_pool) {
                continue;
            }

            if !try_schedule_entity(
                entity,
                &mut scheduled_mask,
                &mut overdue,
                &mut self.scheduled_order,
                &self.rev_cum_deps,
                &self.passes,
                &self.passes_back,
                &mut self.entity_set_pool,
                &self.entity_list_pool,
            ) {
                overdue.insert(entity, &mut self.entity_set_pool);
                continue;
            }

            loop {
                let mut change = false;

                let mut overdue_iter = overdue.iter_detached();
                while let Some(entity) = overdue_iter.next(&self.entity_set_pool) {

                    if scheduled_mask.contains(entity, &self.entity_set_pool) {
                        continue;
                    }

                    if try_schedule_entity(
                        entity,
                        &mut scheduled_mask,
                        &mut overdue,
                        &mut self.scheduled_order,
                        &self.rev_cum_deps,
                        &self.passes,
                        &self.passes_back,
                        &mut self.entity_set_pool,
                        &self.entity_list_pool,
                    ) {
                        change = true;
                    }

                }
                overdue.subtract_from(&scheduled_mask, &mut self.entity_set_pool);

                if !change { break; }
            }

        }

        assert!(overdue.iter(&self.entity_set_pool).count() == 0);

        for n in 0..self.scheduled_order.len() {
            self.schedule_traversal.push(0);
        }

    }

}















