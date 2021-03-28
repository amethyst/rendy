use std::convert::TryInto;

use rendy_core::hal;

use cranelift_entity::EntityList;

use crate::{
    interface::{
        BufferToken, EntityConstructionError, EntityCtx, EntityId, FenceId, GraphCtx, ImageToken,
        NodeConstructionError, PassEntityCtx, PersistenceToken, PersistentBuffer, PersistentImage,
        PersistentKind, Root, SemaphoreId, VirtualId,
    },
    resources::{
        BufferInfo, BufferUsage, ImageInfo, ImageMode, ImageUsage, ProvidedBufferUsage,
        ProvidedImageUsage, VirtualUsage,
    },
    sync::{HasSyncPoint, SyncPoint, SyncPointRef},
    input::ResourceId,
    BufferId, ImageId, IterEither, SchedulerTypes,
};

use super::{
    Attachments, BufferData, BufferSource, BufferUse, BuildKind, Entity, EntityKind, ImageData,
    ImageSource, ImageUsageKind, ImageUse, ProceduralBuilder, RenderPassSpan, Resource,
    ResourceKind, SyncPointKind,
};

impl<T: SchedulerTypes> ProceduralBuilder<T> {
    /// Starts building a pass entity
    pub fn start_pass(&mut self) {
        self.build_status.none();
        self.build_status = BuildKind::Pass;
        self.curr_entity = Some(Entity {
            kind: EntityKind::Pass,
            //images: Vec::new(),
            //buffers: Vec::new(),
            attachments: Some(Attachments {
                depth: None,
                color: Vec::new(),
                input: Vec::new(),
            }),
            node_data: None,
        });
    }

    /// Starts building a standalone entity
    pub fn start_standalone(&mut self) {
        self.build_status.none();
        self.build_status = BuildKind::Standalone;
        self.curr_entity = Some(Entity {
            kind: EntityKind::Standalone,
            //images: Vec::new(),
            //buffers: Vec::new(),
            attachments: None,
            node_data: None,
        });
    }

    /// Commits the entity being built
    pub fn commit(&mut self, data: T::NodeValue) {
        self.build_status.entity();
        self.build_status = BuildKind::None;

        let mut entity = self.curr_entity.take().unwrap();
        entity.node_data = Some(data);
        self.entities.push(entity);
    }

    pub fn remove_data(&mut self, entity: EntityId) -> Option<T::NodeValue> {
        self.entities[entity].node_data.take()
    }

    /// Marks that the given entities are required to be scheduled into the same
    /// render pass. If, for some reason, they can't, a graph validation error
    /// will be emitted.
    ///
    /// If all of the given entities are not render passes, then this will
    /// guarantee failure.
    ///
    /// Order does not matter, they will be scheduled in dependency order.
    pub fn mark_render_pass(&mut self, a: EntityId, b: EntityId) {
        self.render_pass_spans.insert(RenderPassSpan::new(a, b));
    }

    fn image_is_transient(&mut self, image: ImageId) -> bool {
        self.resources[image.into()]
            .kind
            .image_ref()
            .unwrap()
            .info
            .mode
            .is_transient()
    }

    fn handle_image_add_dep(&mut self, image: ImageId) {
        let entity = self.id();
        let num_uses = self.resources[image.into()]
            .kind
            .image_ref()
            .unwrap()
            .uses
            .len();

        if num_uses > 0 {
            let last_use = self.resources[image.into()]
                .kind
                .image_ref()
                .unwrap()
                .last_use()
                .unwrap();

            // If the resource is transient, we need to add it to the render pass.
            if self.image_is_transient(image) {
                self.mark_render_pass(last_use, entity);
            }
        }
    }
}

impl<T: SchedulerTypes> GraphCtx<T> for ProceduralBuilder<T> {
    fn mark_root<R: Into<Root>>(&mut self, root: R) {
        self.roots.insert(root.into());
    }

    fn mark_dead(&mut self, resource: impl Into<ResourceId>) {
        self.resources[resource.into()].is_dead = true;
    }

    fn create_virtual(&mut self) -> VirtualId {
        //self.virtuals.push(VirtualData {
        //    uses: Vec::new(),
        //})
        unimplemented!()
    }

    fn create_image(&mut self, info: ImageInfo) -> ImageId {
        self.resources
            .push(Resource {
                kind: ResourceKind::Image(ImageData {
                    source: ImageSource::Owned,
                    info,
                    uses: Vec::new(),
                }),
                is_dead: false,
                processed_uses: EntityList::new(),
            })
            .into()
    }

    fn provide_image(
        &mut self,
        info: ImageInfo,
        image: impl Into<T::Image>,
        acquire: Option<T::Semaphore>,
        provided_image_usage: Option<ProvidedImageUsage>,
    ) -> ImageId {
        self.resources
            .push(Resource {
                kind: ResourceKind::Image(ImageData {
                    source: ImageSource::Provided {
                        image: Some(image.into()),
                        acquire,
                        provided_image_usage,
                    },
                    info,
                    uses: Vec::new(),
                }),
                is_dead: false,
                processed_uses: EntityList::new(),
            })
            .into()
    }

    fn move_image(&mut self, from: ImageId, to: ImageId) {
        assert!(self.resources[from.into()].kind.alias().is_none());
        assert!(self.resources[to.into()].kind.alias().is_none());
        assert!(
            self.resources[to.into()]
                .kind
                .image_ref()
                .unwrap()
                .uses
                .len()
                == 0
        );

        let mut from_data = ResourceKind::Alias(to.into());
        std::mem::swap(&mut from_data, &mut self.resources[from.into()].kind);
        let mut from_inner = from_data.unwrap_image().unwrap();

        let to_inner = self.resources[to.into()].kind.image_mut().unwrap();

        // Move can only succeed zero or one of the images are provided.
        match (from_inner.source.is_owned(), to_inner.source.is_owned()) {
            (false, true) => to_inner.source = from_inner.source,
            (false, false) => panic!("attempted to perform a move between two non-owned resources"),
            _ => (),
        }

        let provided = to_inner.source.is_owned();

        match (provided, from_inner.info.mode, to_inner.info.mode) {
            // If the image is provided and not transient, we are in trouble.
            (true, ImageMode::Retain { .. }, _) => {
                panic!("attempted to perform a move between provided and retained resources")
            }
            (true, _, ImageMode::Retain { .. }) => {
                panic!("attempted to perform a move between provided and retained resources")
            }

            // If both resources are retained, we are in trouble
            (false, ImageMode::Retain { .. }, ImageMode::Retain { .. }) => {
                panic!("attempted to perform a move between two nontransient resources")
            }

            (false, _, ImageMode::Retain { .. }) => (),
            _ => to_inner.info.mode = from_inner.info.mode,
        }

        match (from_inner.info.kind, to_inner.info.kind) {
            (None, None) => (),
            (Some(l), None) => to_inner.info.kind = Some(l),
            (None, Some(r)) => (),
            (Some(l), Some(r)) if l == r => (),
            (Some(_l), Some(_r)) => panic!("attempted to move between nonmatching image kinds"),
        }

        if from_inner.info.levels != to_inner.info.levels {
            panic!("attempted to move between images with nonmatching levels")
        }

        if from_inner.info.format != to_inner.info.format {
            panic!("attempted to move between images with nonmatching formats")
        }

        // Uses in destination is validated to be empty.
        to_inner.uses = from_inner.uses;
    }

    fn create_buffer(&mut self, info: BufferInfo) -> BufferId {
        self.resources
            .push(Resource {
                kind: ResourceKind::Buffer(BufferData {
                    source: BufferSource::Owned,
                    info,
                    uses: Vec::new(),
                }),
                is_dead: false,
                processed_uses: EntityList::new(),
            })
            .into()
    }

    fn provide_buffer(
        &mut self,
        info: BufferInfo,
        buffer: impl Into<T::Buffer>,
        acquire: Option<T::Semaphore>,
        provided_buffer_usage: Option<ProvidedBufferUsage>,
    ) -> BufferId {
        self.resources
            .push(Resource {
                kind: ResourceKind::Buffer(BufferData {
                    source: BufferSource::Provided {
                        buffer: buffer.into(),
                        acquire,
                        provided_buffer_usage,
                    },
                    info,
                    uses: Vec::new(),
                }),
                is_dead: false,
                processed_uses: EntityList::new(),
            })
            .into()
    }

    fn move_buffer(&mut self, from: BufferId, to: BufferId) {
        assert!(self.resources[from.into()].kind.alias().is_none());
        assert!(self.resources[to.into()].kind.alias().is_none());
        assert!(
            self.resources[to.into()]
                .kind
                .buffer_ref()
                .unwrap()
                .uses
                .len()
                == 0
        );

        // TODO validate kind

        let mut from_data = ResourceKind::Alias(to.into());
        std::mem::swap(&mut from_data, &mut self.resources[from.into()].kind);
        let from_inner = from_data.unwrap_buffer().unwrap();

        let to_inner = self.resources[to.into()].kind.buffer_mut().unwrap();

        to_inner.source = from_inner.source;
        to_inner.info = from_inner.info;
        to_inner.uses = from_inner.uses;
    }

    fn create_persistence_token<K: PersistentKind>(&mut self) -> PersistenceToken<K> {
        unimplemented!()
    }

    fn dispose_persistence_token<K: PersistentKind>(&mut self, token: PersistenceToken<K>) {
        unimplemented!()
    }

    fn sync_point_get<A: HasSyncPoint>(&mut self, a: A) -> SyncPoint {
        let spk = a.into_sync_point();
        match spk {
            SyncPointRef::SyncPoint(sp) => sp,
            SyncPointRef::Image(image) => {
                let idx = self.resources[image.into()].kind.image_ref().unwrap().uses.len();
                self.sync_points
                    .push(SyncPointKind::Resource(image.into(), idx - 1))
            }
            SyncPointRef::Buffer(buffer) => {
                let idx = self.resources[buffer.into()].kind.buffer_ref().unwrap().uses.len();
                self.sync_points
                    .push(SyncPointKind::Resource(buffer.into(), idx - 1))
            }
        }
    }

    fn sync_point_and<A1: HasSyncPoint, A2: HasSyncPoint>(&mut self, a: A1, b: A2) -> SyncPoint {
        let a_sp = self.sync_point_get(a);
        let b_sp = self.sync_point_get(b);

        self.sync_points.push(SyncPointKind::And(a_sp, b_sp))
    }

    fn sync_point_to_semaphore<A: HasSyncPoint>(&mut self, dep: A) -> SemaphoreId {
        let sp = self.sync_point_get(dep);
        self.exported_semaphores.push(sp)
    }

    fn sync_point_to_fence<A: HasSyncPoint>(&mut self, dep: A) -> FenceId {
        let sp = self.sync_point_get(dep);
        self.exported_fences.push(sp)
    }
}

impl<T: SchedulerTypes> EntityCtx<T> for ProceduralBuilder<T> {
    fn id(&self) -> EntityId {
        self.entities.next_key()
    }

    fn use_virtual(&mut self, id: VirtualId, usage: VirtualUsage) {
        //let entity_id = self.id();
        //self.virtuals[id].uses.push((entity_id, usage));
        unimplemented!()
    }

    fn use_image(
        &mut self,
        id: ImageId,
        usage: ImageUsage,
    ) -> Result<ImageToken, EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(id);

        let resource = &mut self.resources[id.into()];
        assert!(!resource.is_dead, "attempted to use dead resource");

        let image = resource.kind.image_mut().unwrap();
        assert!(image.last_use() != Some(entity_id), "an entity can only use a resource once");

        let tok_id = ImageToken(id, image.uses.len().try_into().unwrap());
        image.uses.push(ImageUse {
            usage,
            kind: ImageUsageKind::Use,
            by: entity_id,
        });
        Ok(tok_id)
    }

    fn use_buffer(
        &mut self,
        id: BufferId,
        usage: BufferUsage,
    ) -> Result<BufferToken, EntityConstructionError> {
        let entity_id = self.id();

        let resource = &mut self.resources[id.into()];
        assert!(!resource.is_dead, "attempted to use dead resource");

        let buffer = resource.kind.buffer_mut().unwrap();
        assert!(buffer.last_use() != Some(entity_id), "an entity can only use a resource once");

        let tok_id = BufferToken(id, buffer.uses.len().try_into().unwrap());
        buffer.uses.push(BufferUse {
            usage,
            by: entity_id,
        });
        Ok(tok_id)
    }
}

impl<T: SchedulerTypes> PassEntityCtx<T> for ProceduralBuilder<T> {
    fn use_color(
        &mut self,
        index: usize,
        image_id: ImageId,
        read_access: bool,
    ) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let resource = &mut self.resources[image_id.into()];
        assert!(!resource.is_dead, "attempted to use dead resource");

        let image = resource.kind.image_mut().unwrap();
        assert!(image.last_use() != Some(entity_id), "an entity can only use a resource once");

        let tok_id = ImageToken(image_id, image.uses.len().try_into().unwrap());

        let usage;
        if read_access {
            usage = ImageUsage {
                layout: hal::image::Layout::ColorAttachmentOptimal,
                stages: hal::pso::PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                access: hal::image::Access::COLOR_ATTACHMENT_WRITE,
            };
        } else {
            usage = ImageUsage {
                layout: hal::image::Layout::ColorAttachmentOptimal,
                stages: hal::pso::PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                access: hal::image::Access::COLOR_ATTACHMENT_READ
                    | hal::image::Access::COLOR_ATTACHMENT_WRITE,
            };
        }

        image.uses.push(ImageUse {
            usage,
            kind: ImageUsageKind::Attachment(index),
            by: entity_id,
        });

        self.curr_entity
            .as_mut()
            .unwrap()
            .attachments
            .as_mut()
            .unwrap()
            .color
            .push(tok_id.0);

        Ok(())
    }

    fn use_depth(
        &mut self,
        image_id: ImageId,
        write_access: bool,
    ) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let resource = &mut self.resources[image_id.into()];
        assert!(!resource.is_dead, "attempted to use dead resource");

        let image = resource.kind.image_mut().unwrap();
        assert!(image.last_use() != Some(entity_id), "an entity can only use a resource once");

        let tok_id = ImageToken(image_id, image.uses.len().try_into().unwrap());

        let usage;
        if write_access {
            usage = ImageUsage {
                layout: hal::image::Layout::DepthStencilAttachmentOptimal,
                stages: hal::pso::PipelineStage::EARLY_FRAGMENT_TESTS
                    | hal::pso::PipelineStage::LATE_FRAGMENT_TESTS,
                access: hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ
                    | hal::image::Access::DEPTH_STENCIL_ATTACHMENT_WRITE,
            };
        } else {
            usage = ImageUsage {
                layout: hal::image::Layout::DepthStencilReadOnlyOptimal,
                stages: hal::pso::PipelineStage::EARLY_FRAGMENT_TESTS
                    | hal::pso::PipelineStage::LATE_FRAGMENT_TESTS,
                access: hal::image::Access::DEPTH_STENCIL_ATTACHMENT_READ,
            };
        };

        image.uses.push(ImageUse {
            usage,
            kind: ImageUsageKind::DepthAttachment,
            by: entity_id,
        });

        let mut ntid = Some(tok_id.0);
        std::mem::swap(
            &mut self
                .curr_entity
                .as_mut()
                .unwrap()
                .attachments
                .as_mut()
                .unwrap()
                .depth,
            &mut ntid,
        );
        assert!(ntid.is_none());

        Ok(())
    }

    fn use_input(
        &mut self,
        index: usize,
        image_id: ImageId,
    ) -> Result<(), EntityConstructionError> {
        let entity_id = self.id();
        self.handle_image_add_dep(image_id);

        let resource = &mut self.resources[image_id.into()];
        assert!(!resource.is_dead, "attempted to use dead resource");

        let image = resource.kind.image_mut().unwrap();
        assert!(image.last_use() != Some(entity_id), "an entity can only use a resource once");

        let tok_id = ImageToken(image_id, image.uses.len().try_into().unwrap());

        image.uses.push(ImageUse {
            usage: ImageUsage {
                layout: hal::image::Layout::ShaderReadOnlyOptimal,
                stages: hal::pso::PipelineStage::FRAGMENT_SHADER,
                access: hal::image::Access::INPUT_ATTACHMENT_READ,
            },
            kind: ImageUsageKind::InputAttachment(index),
            by: entity_id,
        });

        self.curr_entity
            .as_mut()
            .unwrap()
            .attachments
            .as_mut()
            .unwrap()
            .input
            .push(tok_id.0);

        Ok(())
    }
}
