use cranelift_entity::SecondaryMap;

use bumpalo::{
    Bump,
    collections::Vec,
};

use log::trace;

use super::{NaturalScheduleMatrix, EntityId, ResourceId, ScheduleAux, Direction};

#[derive(Debug, Clone)]
pub(crate) struct ResourceUse {
    pub(crate) entity: EntityId,
}

#[derive(Debug, Clone)]
pub(crate) enum ResourceUsesKind<'bump> {
    /// At this point in the use chain, one or more reads are performed. These
    /// have no particular defined order relative to each order. The use list
    /// for a resource can never have any concecutive read resource uses.
    Read(Vec<'bump, ResourceUse>),
    /// At this point in the use chain, a write is performed. There can be
    /// multiple concecutive writes to a resource.
    Write(ResourceUse),
}
impl<'bump> ResourceUsesKind<'bump> {
    pub(crate) fn is_write(&self) -> bool {
        match self {
            ResourceUsesKind::Write(_) => true,
            ResourceUsesKind::Read(_) => false,
        }
    }
    pub(crate) fn read(&self) -> &[ResourceUse] {
        match self {
            ResourceUsesKind::Read(inner) => inner,
            _ => unreachable!(),
        }
    }
    pub(crate) fn write(&self) -> &ResourceUse {
        match self {
            ResourceUsesKind::Write(inner) => inner,
            _ => unreachable!(),
        }
    }
    fn read_mut<'a>(&'a mut self) -> &'a mut Vec<'bump, ResourceUse> {
        match self {
            ResourceUsesKind::Read(inner) => inner,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResourceUses<'bump> {
    pub(crate) kind: ResourceUsesKind<'bump>,
    pub(crate) first_entity: EntityId,
}

#[derive(Debug, Clone)]
pub(crate) struct ResourceData<'bump> {
    pub(crate) uses: Vec<'bump, ResourceUses<'bump>>,
}

/// Represents the schedule for all resources in a way where all reads are
/// merged.
/// This data structure will not change due to scheduling, as the only ordering
/// it represents is that imposed by R<>W, W<>R and W<>W hazards.
#[derive(Clone)]
pub(crate) struct OrderIndependentSchedule<'bump> {
    pub(crate) resources: SecondaryMap<ResourceId, ResourceData<'bump>>,
}

impl<'bump> OrderIndependentSchedule<'bump> {

    pub(crate) fn new(schedule: &NaturalScheduleMatrix<EntityId, ResourceId, ScheduleAux>, bump: &'bump Bump) -> Self {
        let mut resources = SecondaryMap::with_default(ResourceData {
            uses: Vec::new_in(bump),
        });

        for resource in schedule.resources() {
            let mut uses = Vec::new_in(bump);
            let mut last_read = false;

            let mut prev_entity = None;
            while let Some((entity, aux)) = schedule.walk_usage(prev_entity, Direction::Forward, resource) {
                prev_entity = Some(entity);

                if aux.usage_kind.is_write() {
                    last_read = false;
                    uses.push(ResourceUses {
                        kind: ResourceUsesKind::Write(ResourceUse {
                            entity,
                        }),
                        first_entity: entity,
                    });
                } else {
                    if !last_read {
                        uses.push(ResourceUses {
                            kind: ResourceUsesKind::Read(Vec::new_in(bump)),
                            first_entity: entity,
                        });
                        last_read = true;
                    }
                    let luv = uses.last_mut().unwrap().kind.read_mut();
                    luv.push(ResourceUse {
                        entity,
                    });
                }
            }
            resources[resource] = ResourceData {
                uses,
            };
        }

        trace!("{:#?}", resources);

        OrderIndependentSchedule {
            resources,
        }
    }
   
}
