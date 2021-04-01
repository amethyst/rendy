use std::collections::BTreeMap;

use rendy_core::hal;

use cranelift_entity::{EntityRef, PrimaryMap, EntityList};

use crate::{
    SchedulerTypes,
    input,
    interface::{EntityId, SemaphoreId, FenceId, ImageId},
    resources::ImageMode,
};
use super::{
    ProceduralBuilder, ResourceKind, ImageUsageKind, EntityKind,
    SyncPoint,
};

impl<T: SchedulerTypes> ProceduralBuilder<T> {

    pub fn postprocess(&mut self) {
        use crate::input::{ResourceUseData, UseKind};

        self.resource_uses.clear();
        self.resource_use_list_pool.clear();

        fn propagate<I: Copy + Eq + Ord>(map: &mut BTreeMap<I, I>) {
            let keys: Vec<_> = map.keys().cloned().collect();
            loop {
                let mut changed = false;

                for key in keys.iter() {
                    let to_1 = map[&key];
                    if let Some(to_2) = map.get(&to_1).cloned() {
                        map.insert(*key, to_2);
                        changed = true;
                    }
                }

                if !changed { break; }
            }
        }

        fn resolve_aliases<I: EntityRef + Ord, T, F>(
            vec: &PrimaryMap<I, T>, resolved: &mut BTreeMap<I, I>, fun: F)
        where
            F: Fn(&T) -> Option<I>,
        {
            debug_assert!(resolved.len() == 0);

            for (id, item) in vec.iter() {
                if let Some(alias) = fun(item) {
                    resolved.insert(id, alias);
                }
            }

            propagate(resolved);
        }

        let mut resolved = BTreeMap::new();
        resolve_aliases(&self.resources, &mut resolved, |data| {
            if let ResourceKind::Alias(to) = &data.kind {
                Some(*to)
            } else {
                None
            }
        });

        // TODO we need to make sure aliases get propagated properly

        for (res_id, resource) in self.resources.iter_mut() {
            //let idx = input.resource.push(ResourceData {
            //    //uses: EntitySet::new(),
            //    uses_l: EntityList::new(),
            //    aux: (),
            //});
            //assert!(o_idx.index() == idx.index());

            resource.processed_uses = EntityList::new();

            match &resource.kind {
                ResourceKind::Alias(_) => (),
                ResourceKind::Image(data) => {
                    debug_assert!(data.uses.windows(2).all(|w| w[0].by.index() <= w[1].by.index()));

                    for use_data in data.uses.iter() {
                        let entity_id = EntityId::new(use_data.by.index());
                        let layout = use_data.usage.layout;
                        let (use_kind, is_write) = match use_data.kind {
                            ImageUsageKind::InputAttachment(_) => {
                                assert!(!use_data.usage.is_write());
                                (UseKind::Attachment(layout), false)
                            },
                            ImageUsageKind::DepthAttachment => {
                                (UseKind::Attachment(layout), true)
                            },
                            ImageUsageKind::Attachment(_) => {
                                (UseKind::Attachment(layout), true)
                            },
                            ImageUsageKind::Use => {
                                (UseKind::Use, use_data.usage.is_write())
                            },
                        };

                        let resource_use = self.resource_uses.push(ResourceUseData {
                            entity: entity_id,
                            resource: res_id,
                            use_kind,
                            is_write,
                            stages: hal::pso::PipelineStage::BOTTOM_OF_PIPE | hal::pso::PipelineStage::TOP_OF_PIPE,
                            specific_use_data: input::SpecificResourceUseData::Image {
                                state: (use_data.usage.access, use_data.usage.layout),
                            },
                        });
                        resource.processed_uses.push(
                            resource_use, &mut self.resource_use_list_pool);
                    }
                },
                ResourceKind::Buffer(data) => {
                    debug_assert!(data.uses.windows(2).all(|w| w[0].by.index() <= w[1].by.index()));

                    for use_data in data.uses.iter() {
                        let entity_id = EntityId::new(use_data.by.index());
                        let resource_use = self.resource_uses.push(ResourceUseData {
                            entity: entity_id,
                            resource: res_id,
                            use_kind: UseKind::Use,
                            is_write: use_data.usage.is_write(),
                            stages: hal::pso::PipelineStage::BOTTOM_OF_PIPE | hal::pso::PipelineStage::TOP_OF_PIPE,
                            specific_use_data: input::SpecificResourceUseData::Buffer {
                                state: use_data.usage.access,
                            },
                        });
                        resource.processed_uses.push(
                            resource_use, &mut self.resource_use_list_pool);
                    }
                },
            }
        }

        for (entity_id, entity_data) in self.entities.iter_mut() {
            if let Some(attachments) = &mut entity_data.attachments {
                let map_img = |img: ImageId| -> ImageId {
                    resolved
                        .get(&img.into())
                        .map(|v| (*v).into())
                        .unwrap_or(img)
                };

                if let Some(depth) = &mut attachments.depth {
                    *depth = map_img(*depth);
                }
                for color in attachments.color.iter_mut() {
                    *color = map_img(*color);
                }
                for input in attachments.input.iter_mut() {
                    *input = map_img(*input);
                }
            }
        }
    }

}

impl<T: SchedulerTypes> input::SchedulerInput for ProceduralBuilder<T> {
    fn num_entities(&self) -> usize {
        self.entities.len()
    }
    fn num_resources(&self) -> usize {
        self.resources.len()
    }
    fn get_uses(&self, resource_id: input::ResourceId) -> &[input::ResourceUseId] {
        let resource = &self.resources[resource_id];
        resource.processed_uses.as_slice(&self.resource_use_list_pool)
    }
    fn resource_use_data(&self, resource_use: input::ResourceUseId) -> input::ResourceUseData {
        self.resource_uses[resource_use]
    }
    fn resource_data(&self, resource_id: input::ResourceId) -> input::ResourceData {
        let resource = &self.resources[resource_id];
        match &resource.kind {
            ResourceKind::Image(image) => {
                let info = image.info;

                let load_op = match info.mode {
                    ImageMode::Retain { .. } => todo!(),
                    ImageMode::DontCare { .. } => hal::pass::AttachmentLoadOp::DontCare,
                    ImageMode::Clear { .. } => hal::pass::AttachmentLoadOp::Clear,
                };

                let data = input::ImageData {
                    load_op,
                    used_after: true,
                    kind: info.kind,
                    format: info.format,
                    usage: image.source.initial_usage(),
                };
                input::ResourceData::Image(data)
            },
            ResourceKind::Buffer(buffer) => {
                todo!()
            },
            _ => unreachable!(),
        }
    }
    fn get_render_pass_spans(&self, out: &mut Vec<input::RenderPassSpan>) {
        out.clear();
        out.extend(self.render_pass_spans.iter().cloned());
    }
    fn entity_kind(&self, entity_id: input::EntityId) -> input::EntityKind {
        let entity = &self.entities[entity_id];
        match entity.kind {
            EntityKind::Pass => input::EntityKind::Pass,
            EntityKind::Transfer => input::EntityKind::Transfer,
            EntityKind::Standalone => input::EntityKind::Standalone,
        }
    }
    fn has_aquire_semaphore(&self, resource: input::ResourceId) -> Option<()> {
        todo!()
    }
    fn num_semaphores(&self) -> usize {
        self.exported_semaphores.len()
    }
    fn get_semaphore(&self, semaphore: SemaphoreId) -> SyncPoint {
        self.exported_semaphores[semaphore]
    }
    fn num_fences(&self) -> usize {
        self.exported_fences.len()
    }
    fn get_fence(&self, fence: FenceId) -> SyncPoint {
        self.exported_fences[fence]
    }
    fn get_sync_point(&self, sync_point: SyncPoint) -> input::SyncPointKind {
        self.sync_points[sync_point]
    }
}
