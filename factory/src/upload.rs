
use std::collections::VecDeque;

use crate::{
    command::{Family, CommandBuffer, CommandPool, Transfer, IndividualReset, PendingOnceState, InitialState, RecordingState, OneShot, PrimaryLevel, QueueId, Submission},
    resource::{Buffer, Image},
};

/// State of the buffer on device.
#[derive(Clone, Copy, Debug)]
pub struct BufferState {
    /// Queue that uses the buffer.
    pub queue: QueueId,

    /// Stages when buffer get used.
    pub stage: gfx_hal::pso::PipelineStage,

    /// Access performed by device.
    pub access: gfx_hal::buffer::Access,
}

impl BufferState {
    /// Create default buffet state.
    pub fn new(queue: QueueId) -> Self {
        BufferState {
            queue,
            stage: gfx_hal::pso::PipelineStage::TOP_OF_PIPE,
            access: gfx_hal::buffer::Access::all(),
        }
    }

    /// Set specific stage.
    pub fn with_stage(mut self, stage: gfx_hal::pso::PipelineStage) -> Self {
        self.stage = stage;
        self
    }

    /// Set specific access.
    pub fn with_access(mut self, access: gfx_hal::buffer::Access) -> Self {
        self.access = access;
        self
    }
}

/// State of the image on device.
#[derive(Clone, Copy, Debug)]
pub struct ImageState {
    /// Queue that uses the image.
    pub queue: QueueId,

    /// Stages when image get used.
    pub stage: gfx_hal::pso::PipelineStage,

    /// Access performed by device.
    pub access: gfx_hal::image::Access,

    /// Layout in which image is accessed.
    pub layout: gfx_hal::image::Layout,
}

impl ImageState {
    /// Create default buffet state.
    pub fn new(queue: QueueId, layout: gfx_hal::image::Layout) -> Self {
        ImageState {
            queue,
            stage: gfx_hal::pso::PipelineStage::TOP_OF_PIPE,
            access: gfx_hal::image::Access::all(),
            layout,
        }
    }

    /// Set specific stage.
    pub fn with_stage(mut self, stage: gfx_hal::pso::PipelineStage) -> Self {
        self.stage = stage;
        self
    }

    /// Set specific access.
    pub fn with_access(mut self, access: gfx_hal::image::Access) -> Self {
        self.access = access;
        self
    }
}

/// Either image state or just layout for image that is not used by device.
#[derive(Clone, Copy, Debug)]
pub enum ImageStateOrLayout {
    /// State of image used by device.
    State(ImageState),

    /// Layout of image not used by device.
    Layout(gfx_hal::image::Layout),
}

impl ImageStateOrLayout {
    /// Create instance that descibes unused image with undefined content
    /// (or if previous content doesn't need to be preserved).
    /// This can be used for newly created images.
    /// Or when whole image is updated.
    pub fn undefined() -> Self {
        ImageStateOrLayout::Layout(gfx_hal::image::Layout::Undefined)
    }
}

impl From<ImageState> for ImageStateOrLayout {
    fn from(state: ImageState) -> Self {
        ImageStateOrLayout::State(state)
    }
}

impl From<gfx_hal::image::Layout> for ImageStateOrLayout {
    fn from(layout: gfx_hal::image::Layout) -> Self {
        ImageStateOrLayout::Layout(layout)
    }
}

#[derive(Debug)]
pub(crate) struct Uploader<B: gfx_hal::Backend> {
    pub(crate) families: Vec<parking_lot::Mutex<FamilyUploads<B>>>,
}

impl<B> Uploader<B>
where
    B: gfx_hal::Backend,
{
    pub(crate) fn new(families: usize) -> Self {
        Uploader {
            families: (0 .. families).map(|_| parking_lot::Mutex::new(FamilyUploads {
                    fences: Vec::new(),
                    pool: None,
                    next: Vec::new(),
                    pending: VecDeque::new(),
                    command_buffers: Vec::new(),
                })).collect(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct FamilyUploads<B: gfx_hal::Backend> {
    pool: Option<CommandPool<B, Transfer, IndividualReset>>,
    command_buffers: Vec<CommandBuffer<B, Transfer, InitialState, PrimaryLevel, IndividualReset>>,
    next: Vec<Option<NextUploads<B>>>,
    pending: VecDeque<PendingUploads<B>>,
    fences: Vec<B::Fence>,
}

#[derive(Debug)]
pub(crate) struct PendingUploads<B: gfx_hal::Backend> {
    command_buffer: CommandBuffer<B, Transfer, PendingOnceState, PrimaryLevel, IndividualReset>,
    staging_buffers: Vec<Buffer<B>>,
    fence: B::Fence,
}

#[derive(Debug)]
struct NextUploads<B: gfx_hal::Backend> {
    command_buffer: CommandBuffer<B, Transfer, RecordingState<OneShot>, PrimaryLevel, IndividualReset>,
    staging_buffers: Vec<Buffer<B>>,
    fence: B::Fence,
}

impl<B> FamilyUploads<B>
where
    B: gfx_hal::Backend,
{
    pub(crate) unsafe fn upload_buffer(
        &mut self,
        device: &B::Device,
        family: &Family<B>,
        buffer: &mut Buffer<B>,
        offset: u64,
        staging: Buffer<B>,
        last: Option<BufferState>,
        next: BufferState,
    ) ->Result<(), failure::Error> {
        let next_upload = self.next_upload(device, family, next.queue.index())?;
        let mut encoder = next_upload.command_buffer.encoder();
        let range = Some(offset) .. Some(offset + staging.size());

        if let Some(last) = last {
            if last.queue != next.queue {
                unimplemented!("Can't sync resources across queues");
            }

            encoder.pipeline_barrier(
                last.stage .. gfx_hal::pso::PipelineStage::TRANSFER,
                gfx_hal::memory::Dependencies::empty(),
                Some(gfx_hal::memory::Barrier::Buffer {
                    states: last.access .. gfx_hal::buffer::Access::TRANSFER_WRITE,
                    target: buffer.raw(),
                    families: None,
                    range: range.clone(),
                }),
            );
        }

        encoder.copy_buffer(
            staging.raw(),
            buffer.raw(),
            Some(gfx_hal::command::BufferCopy {
                src: 0,
                dst: offset,
                size: staging.size(),
            })
        );

        encoder.pipeline_barrier(
            gfx_hal::pso::PipelineStage::TRANSFER .. next.stage,
            gfx_hal::memory::Dependencies::empty(),
            Some(gfx_hal::memory::Barrier::Buffer {
                states: gfx_hal::buffer::Access::TRANSFER_WRITE .. next.access,
                target: buffer.raw(),
                families: None,
                range: range.clone(),
            }),
        );

        next_upload.staging_buffers.push(staging);

        Ok(())
    }

    pub(crate) unsafe fn upload_image(
        &mut self,
        device: &B::Device,
        family: &Family<B>,
        image: &mut Image<B>,
        data_width: u32,
        data_height: u32,
        image_layers: gfx_hal::image::SubresourceLayers,
        image_offset: gfx_hal::image::Offset,
        image_extent: gfx_hal::image::Extent,
        staging: Buffer<B>,
        last: ImageStateOrLayout,
        next: ImageState,
    ) -> Result<(), failure::Error> {
        let next_upload = self.next_upload(device, family, next.queue.index())?;

        let whole_image = image_offset == gfx_hal::image::Offset::ZERO && image_extent == image.kind().extent();

        let image_range = gfx_hal::image::SubresourceRange {
            aspects: image_layers.aspects,
            levels: image_layers.level .. image_layers.level + 1,
            layers: image_layers.layers.clone(),
        };

        let mut encoder = next_upload.command_buffer.encoder();

        match last.into() {
            ImageStateOrLayout::State(last) => {
                if last.queue != next.queue {
                    unimplemented!("Can't sync resources across queues");
                }

                let last_layout = if whole_image {
                    gfx_hal::image::Layout::Undefined
                } else {
                    last.layout
                };

                encoder.pipeline_barrier(
                    last.stage .. gfx_hal::pso::PipelineStage::TRANSFER,
                    gfx_hal::memory::Dependencies::empty(),
                    Some(gfx_hal::memory::Barrier::Image {
                        states: (last.access, last_layout) .. (gfx_hal::image::Access::TRANSFER_WRITE, gfx_hal::image::Layout::TransferDstOptimal),
                        target: image.raw(),
                        families: None,
                        range: image_range.clone(),
                    }),
                );
            }
            ImageStateOrLayout::Layout(mut last_layout) => {
                if last_layout != gfx_hal::image::Layout::TransferDstOptimal {
                    if whole_image {
                        last_layout = gfx_hal::image::Layout::Undefined;
                    }
                    encoder.pipeline_barrier(
                        gfx_hal::pso::PipelineStage::TOP_OF_PIPE .. gfx_hal::pso::PipelineStage::TRANSFER,
                        gfx_hal::memory::Dependencies::empty(),
                        Some(gfx_hal::memory::Barrier::Image {
                            states: (gfx_hal::image::Access::empty(), last_layout) .. (gfx_hal::image::Access::TRANSFER_WRITE, gfx_hal::image::Layout::TransferDstOptimal),
                            target: image.raw(),
                            families: None,
                            range: image_range.clone(),
                        }),
                    );
                }
            }
        }

        encoder.copy_buffer_to_image(
            staging.raw(),
            image.raw(),
            gfx_hal::image::Layout::TransferDstOptimal,
            Some(gfx_hal::command::BufferImageCopy {
                buffer_offset: 0,
                buffer_width: data_width,
                buffer_height: data_height,
                image_layers,
                image_offset,
                image_extent,
            })
        );

        // TODO: batch barriers.
        encoder.pipeline_barrier(
            gfx_hal::pso::PipelineStage::TRANSFER .. next.stage,
            gfx_hal::memory::Dependencies::empty(),
            Some(gfx_hal::memory::Barrier::Image {
                states: (gfx_hal::image::Access::TRANSFER_WRITE, gfx_hal::image::Layout::TransferDstOptimal) .. (next.access, next.layout),
                target: image.raw(),
                families: None,
                range: image_range.clone(),
            }),
        );

        next_upload.staging_buffers.push(staging);

        Ok(())
    }

    pub(crate) unsafe fn flush(&mut self, family: &mut Family<B>) {
        for (queue, next) in self.next.drain(..).enumerate().filter_map(|(i, x)| x.map(|x| (i, x))) {
            let (submit, command_buffer) = next.command_buffer.finish().submit_once();

            family.queues_mut()[queue].submit(Some(Submission::new().submits(Some(submit))), Some(&next.fence));
            
            self.pending.push_back(PendingUploads {
                command_buffer,
                staging_buffers: next.staging_buffers,
                fence: next.fence,
            });
        }
    }

    unsafe fn next_upload(&mut self, device: &B::Device, family: &Family<B>, queue: usize) -> Result<&mut NextUploads<B>, failure::Error> {
        while self.next.len() <= queue {
            self.next.push(None);
        }

        let pool = &mut self.pool;

        match &mut self.next[queue] {
            Some(next) => Ok(next),
            slot @ None => {
                let buffer = self.command_buffers.pop()
                    .map(Ok)
                    .unwrap_or_else(|| -> Result<_, failure::Error> {
                        let pool = match pool {
                            Some(pool) => pool,
                            slot @ None => {
                                let pool = family.create_pool(device)?;
                                *slot = Some(pool.with_capability().unwrap());
                                slot.as_mut().unwrap()
                            }
                        };

                        Ok(pool.allocate_buffers(1).pop().unwrap())
                    })?;

                let fence = self.fences.pop().map(Ok).unwrap_or_else(|| gfx_hal::Device::create_fence(device, false))?;
                *slot = Some(NextUploads {
                    command_buffer: buffer.begin(OneShot, ()),
                    staging_buffers: Vec::new(),
                    fence,
                });

                Ok(slot.as_mut().unwrap())
            }
        }
    }

    /// Cleanup pending updates.
    /// 
    /// # Safety
    /// 
    /// `device` must be the same that was used with other methods of this instance.
    /// 
    pub(crate) unsafe fn cleanup(&mut self, device: &B::Device) {
        while let Some(pending) = self.pending.pop_front() {
            match gfx_hal::Device::get_fence_status(device, &pending.fence) {
                Ok(false) => {
                    self.pending.push_front(pending);
                    return;
                }
                Err(gfx_hal::device::DeviceLost) => {
                    self.pending.push_front(pending);
                    panic!("Device lost error is not handled yet");
                }
                Ok(true) => {
                    self.fences.push(pending.fence);
                    self.command_buffers.push(pending.command_buffer.mark_complete().reset());
                }
            }
        }
    }

    /// # Safety
    /// 
    /// Device must be idle.
    /// 
    pub(crate) unsafe fn dispose(mut self, device: &B::Device) {
        let pool = &mut self.pool;
        self.pending.drain(..).for_each(|pending| {
            gfx_hal::Device::destroy_fence(device, pending.fence);
            pool.as_mut().unwrap().free_buffers(Some(pending.command_buffer.mark_complete()))
        });

        self.fences.drain(..).for_each(|fence| gfx_hal::Device::destroy_fence(device, fence));
        self.command_buffers.drain(..).for_each(|command_buffer| pool.as_mut().unwrap().free_buffers(Some(command_buffer)));
        self.next.drain(..).filter_map(|n|n).for_each(|next| {
            pool.as_mut().unwrap().free_buffers(Some(next.command_buffer));
            gfx_hal::Device::destroy_fence(device, next.fence);
        });
        drop(pool);
        self.pool.map(|pool| pool.dispose(device));
    }
}
