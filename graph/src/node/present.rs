//! Defines present node.

use crate::{
    ImageId, BufferId, NodeId,
    command::{CommandPool, CommandBuffer, ExecutableState, PendingState, MultiShot, SimultaneousUse, Family, Submission, Submit, FamilyId, QueueId},
    factory::Factory,
    frame::Frames,
    wsi::{Surface, Target, Backbuffer},
    node::{NodeBuilder, DynNode, NodeImage, NodeBuffer, ImageAccess, gfx_acquire_barriers, gfx_release_barriers, BufferAccess},
};

#[derive(Debug)]
struct ForImage<B: gfx_hal::Backend> {
    acquire: B::Semaphore,
    release: B::Semaphore,
    submit: Submit<B, SimultaneousUse>,
    buffer: CommandBuffer<B, gfx_hal::QueueType, PendingState<ExecutableState<MultiShot<SimultaneousUse>>>>,
}

/// Node that presents images to the surface.
#[derive(Debug)]
pub struct PresentNode<B: gfx_hal::Backend> {
    per_image: Vec<ForImage<B>>,
    free: Option<B::Semaphore>,
    target: Target<B>,
    pool: CommandPool<B, gfx_hal::QueueType>,
}

impl<B> PresentNode<B>
where
    B: gfx_hal::Backend,
{
    /// Node builder.
    pub fn builder(surface: Surface<B>, image: ImageId) -> PresentBuilder<B> {
        PresentBuilder {
            surface,
            image,
            dependencies: Vec::new()
        }
    }
}

/// Presentation node description.
#[derive(Debug)]
pub struct PresentBuilder<B: gfx_hal::Backend> {
    surface: Surface<B>,
    image: ImageId,
    dependencies: Vec<NodeId>,
}

impl<B> PresentBuilder<B>
where
    B: gfx_hal::Backend,
{
    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn add_dependency(&mut self, dependency: NodeId) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: NodeId) -> Self {
        self.add_dependency(dependency);
        self
    }
}

impl<B, T> NodeBuilder<B, T> for PresentBuilder<B>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    fn family(&self, families: &[Family<B>]) -> Option<FamilyId> {
        // Find correct queue family.
        families.get(0).map(Family::index)
    }

    fn buffers(&self) -> Vec<(BufferId, BufferAccess)> {
        Vec::new()
    }

    fn images(&self) -> Vec<(ImageId, ImageAccess)> {
        vec![(self.image, ImageAccess {
            access: gfx_hal::image::Access::TRANSFER_READ,
            layout: gfx_hal::image::Layout::TransferSrcOptimal,
            usage: gfx_hal::image::Usage::TRANSFER_SRC,
            stages: gfx_hal::pso::PipelineStage::TRANSFER,
        })]
    }

    fn dependencies(&self) -> Vec<NodeId> {
        self.dependencies.clone()
    }

    fn build<'a>(
        self: Box<Self>,
        factory: &mut Factory<B>,
        _aux: &mut T,
        family: FamilyId,
        buffers: Vec<NodeBuffer<'a, B>>,
        images: Vec<NodeImage<'a, B>>,
    ) -> Result<Box<dyn DynNode<B, T>>, failure::Error> {
        assert_eq!(buffers.len(), 0);
        assert_eq!(images.len(), 1);

        let ref input_image = images[0];
        let target = factory.create_target(self.surface, 3, gfx_hal::image::Usage::TRANSFER_DST)?;
        let mut pool = factory.create_command_pool(family)?;

        let per_image = match target.backbuffer() {
            Backbuffer::Images(target_images) => {
                let buffers = pool.allocate_buffers(target_images.len());
                target_images.iter().zip(buffers).map(|(target_image, buf_initial)| {
                    let mut buf_recording = buf_initial.begin(MultiShot(SimultaneousUse), ());
                    let mut encoder = buf_recording.encoder();
                    {
                        let (stages, barriers) = gfx_acquire_barriers(None, Some(input_image));
                        log::info!("Acquire {:?} : {:#?}", stages, barriers);
                        encoder.pipeline_barrier(
                            stages,
                            gfx_hal::memory::Dependencies::empty(),
                            barriers,
                        );
                    }
                    encoder.copy_image(
                        input_image.image.raw(),
                        input_image.layout,
                        target_image.raw(),
                        gfx_hal::image::Layout::TransferDstOptimal,
                        Some(gfx_hal::command::ImageCopy {
                            src_subresource: gfx_hal::image::SubresourceLayers {
                                aspects: gfx_hal::format::Aspects::COLOR,
                                level: 0,
                                layers: 0..1,
                            },
                            src_offset: gfx_hal::image::Offset::ZERO,
                            dst_subresource: gfx_hal::image::SubresourceLayers {
                                aspects: gfx_hal::format::Aspects::COLOR,
                                level: 0,
                                layers: 0..1,
                            },
                            dst_offset: gfx_hal::image::Offset::ZERO,
                            extent: gfx_hal::image::Extent {
                                width: target_image.kind().extent().width,
                                height: target_image.kind().extent().height,
                                depth: 1,
                            },
                        }),
                    );
                    {
                        let (stages, barriers) = gfx_release_barriers(None, Some(input_image));
                        log::info!("Release {:?} : {:#?}", stages, barriers);
                        encoder.pipeline_barrier(
                            stages,
                            gfx_hal::memory::Dependencies::empty(),
                            barriers,
                        );
                    }

                    let (submit, buffer) = buf_recording.finish().submit();

                    ForImage {
                        submit,
                        buffer,
                        acquire: factory.create_semaphore().unwrap(),
                        release: factory.create_semaphore().unwrap(),
                    }
                }).collect()
            }
            _ => unimplemented!(),
        };

        Ok(Box::new(PresentNode {
            free: Some(factory.create_semaphore().unwrap()),
            target,
            pool,
            per_image,
        }))
    }
}

impl<B, T> DynNode<B, T> for PresentNode<B>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    unsafe fn run<'a>(
        &mut self,
        factory: &mut Factory<B>,
        _aux: &mut T,
        _frames: &Frames<B>,
        qid: QueueId,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&B::Fence>,
    ) {
        let acquire = self.free.take().unwrap();
        let next = self.target.next_image(&acquire).unwrap();
        log::trace!("Present: {:#?}", next);
        let ref mut for_image = self.per_image[next[0] as usize];
        self.free = Some(std::mem::replace(&mut for_image.acquire, acquire));

        let family = factory.family_mut(qid.family());

        family.submit(
            qid.index(),
            Some(
                Submission::new()
                    .submits(Some(&for_image.submit))
                    .wait(waits.iter().cloned().chain(Some((&for_image.acquire, gfx_hal::pso::PipelineStage::TRANSFER))))
                    .signal(signals.iter().cloned().chain(Some(&for_image.release)))
            ),
            fence,
        );

        next.present(&mut family.queues_mut()[qid.index()], Some(&for_image.release))
            .expect("Fix swapchain error");
    }

    unsafe fn dispose(mut self: Box<Self>, factory: &mut Factory<B>, _aux: &mut T) {
        for for_image in self.per_image {
            drop(for_image.submit);
            factory.destroy_semaphore(for_image.acquire);
            factory.destroy_semaphore(for_image.release);
            self.pool.free_buffers(Some(for_image.buffer.mark_complete()));
        }

        factory.destroy_semaphore(self.free.unwrap());
        factory.destroy_command_pool(self.pool);
        factory.destroy_target(self.target);
    }
}
