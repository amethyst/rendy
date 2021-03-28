use std::ops::Range;

use crate::{Scheduler, RenderPass, ScheduleEntry, EntityId};

pub struct ScheduleIterator {
    pub schedule_idx: usize,
}

pub enum Current {
    Pass(CurrentPass),
    General(CurrentGeneral),
}

pub struct CurrentPass {
    pub pass: RenderPass,
    pub schedule_entries: PassEntriesIterator,
}

pub struct CurrentGeneral {
    pub entity: EntityId,
}

impl ScheduleIterator {

    pub fn new() -> Self {
        ScheduleIterator {
            schedule_idx: 0,
        }
    }

    pub fn next(&mut self, scheduler: &Scheduler) -> Option<Current> {
        let entry = scheduler.scheduled_order.get(self.schedule_idx)?;
        let current = match entry {
            ScheduleEntry::General(entity) => {
                self.schedule_idx += 1;
                Current::General(CurrentGeneral {
                    entity: *entity,
                })
            },
            ScheduleEntry::PassEntity(_entity, pass, _subpass_idx) => {
                let num = scheduler
                    .scheduled_order[self.schedule_idx..]
                    .iter()
                    .enumerate()
                    .take_while(|(idx, schedule_entry)| {
                        match schedule_entry {
                            ScheduleEntry::PassEntity(_entity, i_pass, subpass_idx) if *i_pass == *pass => {
                                assert!(*subpass_idx == *idx);
                                true
                            },
                            _ => false,
                        }
                    })
                    .count();

                let schedule_entries = PassEntriesIterator {
                    current: self.schedule_idx,
                    end: self.schedule_idx + num,
                };

                self.schedule_idx += num;

                Current::Pass(CurrentPass {
                    pass: *pass,
                    schedule_entries,
                })
            },
        };
        Some(current)
    }

    pub fn next_idx(&self) -> usize {
        self.schedule_idx
    }

}

pub struct PassEntriesIterator {
    current: usize,
    end: usize,
}
impl Iterator for PassEntriesIterator {
    type Item = (usize, bool);
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let ret = (self.current, self.current + 1 < self.end);
            self.current += 1;
            Some(ret)
        } else {
            None
        }
    }
}
