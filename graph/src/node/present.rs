//! Defines present node.

use crate::{
    chain::QueueId,
    command::{CommandPool, CommandBuffer, ExecutableState, PendingState, PrimaryLevel, MultiShot, SimultaneousUse, Encoder, Family, Submission, Submit},
    factory::Factory,
    frame::Frames,
    wsi::{Surface, Target},
    node::{AnyNodeDesc, AnyNode, NodeImage, NodeBuffer, NodeBuilder, ImageAccess},
};

#[derive(Debug)]
struct ForImage<B: gfx_hal::Backend> {
    acquire: B::Semaphore,
    release: B::Semaphore,
    submit: Submit<'static, B, SimultaneousUse>,
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
    pub fn builder<T: ?Sized>(surface: Surface<B>) -> NodeBuilder<B, T> {
        PresentDesc::new(surface).builder()
    }
}

/// Presentation node description.
#[derive(Debug)]
pub struct PresentDesc<B: gfx_hal::Backend> {
    surface: Surface<B>,
}

impl<B> PresentDesc<B>
where
    B: gfx_hal::Backend,
{
    /// Create present builder
    pub fn new(
        surface: Surface<B>,
    ) -> Self {
        PresentDesc {
            surface,
        }
    }
}

impl<B, T> AnyNodeDesc<B, T> for PresentDesc<B>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    fn family(&self, families: &[Family<B>]) -> Option<gfx_hal::queue::QueueFamilyId> {
        families.get(0).map(Family::index)
    }

    fn images(&self) -> Vec<ImageAccess> {
        vec![ImageAccess {
            access: gfx_hal::image::Access::TRANSFER_READ,
            layout: gfx_hal::image::Layout::TransferSrcOptimal,
            usage: gfx_hal::image::Usage::TRANSFER_SRC,
            stages: gfx_hal::pso::PipelineStage::TRANSFER,
        }]
    }

    fn build<'a>(
        self: Box<Self>,
        factory: &mut Factory<B>,
        _aux: &mut T,
        family: gfx_hal::queue::QueueFamilyId,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
    ) -> Result<Box<dyn AnyNode<B, T>>, failure::Error> {
        assert_eq!(buffers.len(), 0);
        assert_eq!(images.len(), 1);

        let ref input_image = images[0];
        let target = factory.create_target(self.surface, 3, gfx_hal::image::Usage::TRANSFER_DST)?;
        let mut pool = factory.create_command_pool(family, ())?;

        let per_image = match target.backbuffer() {
            gfx_hal::Backbuffer::Images(target_images) => {
                let buffers = pool.allocate_buffers(PrimaryLevel, target_images.len());
                target_images.iter().zip(buffers).map(|(target_image, initial)| {
                    let mut encoder = initial.begin(MultiShot(SimultaneousUse), ());
                    encoder.copy_image(
                        input_image.image.raw(),
                        input_image.layout,
                        &target_image,
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
                                width: target.extent().width,
                                height: target.extent().height,
                                depth: 1,
                            },
                        }),
                    );

                    let (submit, buffer) = encoder.finish().submit();

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

impl<B, T> AnyNode<B, T> for PresentNode<B>
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
        unsafe {
            let next = self.target.next_image(&acquire).unwrap();
            log::trace!("Present: {:#?}", next);
            let ref mut for_image = self.per_image[next[0] as usize];
            self.free = Some(std::mem::replace(&mut for_image.acquire, acquire));

            let family = factory.family_mut(qid.family());

            family.submit(
                qid.index(),
                Some(Submission {
                    waits: waits.iter().cloned().chain(Some((&for_image.acquire, gfx_hal::pso::PipelineStage::TRANSFER))),
                    signals: signals.iter().cloned().chain(Some(&for_image.release)),
                    submits: Some(&for_image.submit),
                }),
                fence,
            );

            next.present(&mut family.queues_mut()[qid.index()], Some(&for_image.release)).unwrap();
        }
    }

    unsafe fn dispose(self: Box<Self>, _factory: &mut Factory<B>, _aux: &mut T) {
        unimplemented!()
    }
}
