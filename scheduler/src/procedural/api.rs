use crate::{input::ResourceId, resources::ImageInfo, EntityId, ImageId, SchedulerTypes};

use super::{Attachments, ProceduralBuilder, Resource, ResourceKind};

impl<T: SchedulerTypes> ProceduralBuilder<T> {
    pub fn get_attachments(&self, entity_id: EntityId) -> Option<&Attachments> {
        self.entities[entity_id].attachments.as_ref()
    }

    pub fn get_image_info(&self, image_id: ImageId) -> ImageInfo {
        match &self.resources[image_id.into()].kind {
            ResourceKind::Image(image) => image.info.clone(),
            _ => panic!(),
        }
    }

    pub fn get_resource_info_mut(&mut self, resource_id: ResourceId) -> &mut Resource<T> {
        &mut self.resources[resource_id]
    }
}
