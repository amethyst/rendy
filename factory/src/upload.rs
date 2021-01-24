use std::{collections::VecDeque, iter::once};

use hal::device::{Device as _, OutOfMemory};
use rendy_core::{hal, Device};

use crate::{
    barriers::Barriers,
    command::{
        CommandBuffer, CommandPool, Families, Family, IndividualReset, InitialState, OneShot,
        PendingOnceState, PrimaryLevel, QueueId, RecordingState, Submission, Transfer,
    },
    resource::{Buffer, Escape, Handle, Image},
};

/// State of the buffer on device.
#[derive(Clone, Copy, Debug)]
pub struct BufferState {
    /// Queue that uses the buffer.
    pub queue: QueueId,

    /// Stages when buffer get used.
    pub stage: hal::pso::PipelineStage,

    /// Access performed by device.
    pub access: hal::buffer::Access,
}

impl BufferState {
    /// Create default buffet state.
    pub fn new(queue: QueueId) -> Self {
        BufferState {
            queue,
            stage: hal::pso::PipelineStage::TOP_OF_PIPE,
            access: hal::buffer::Access::all(),
        }
    }

    /// Set specific stage.
    pub fn with_stage(mut self, stage: hal::pso::PipelineStage) -> Self {
        self.stage = stage;
        self
    }

    /// Set specific access.
    pub fn with_access(mut self, access: hal::buffer::Access) -> Self {
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
    pub stage: hal::pso::PipelineStage,

    /// Access performed by device.
    pub access: hal::image::Access,

    /// Layout in which image is accessed.
    pub layout: hal::image::Layout,
}

impl ImageState {
    /// Create default buffet state.
    pub fn new(queue: QueueId, layout: hal::image::Layout) -> Self {
        ImageState {
            queue,
            stage: hal::pso::PipelineStage::TOP_OF_PIPE,
            access: hal::image::Access::all(),
            layout,
        }
    }

    /// Set specific stage.
    pub fn with_stage(mut self, stage: hal::pso::PipelineStage) -> Self {
        self.stage = stage;
        self
    }

    /// Set specific access.
    pub fn with_access(mut self, access: hal::image::Access) -> Self {
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
    Layout(hal::image::Layout),
}

impl ImageStateOrLayout {
    /// Create instance that descibes unused image with undefined content
    /// (or if previous content doesn't need to be preserved).
    /// This can be used for newly created images.
    /// Or when whole image is updated.
    pub fn undefined() -> Self {
        ImageStateOrLayout::Layout(hal::image::Layout::Undefined)
    }
}

impl From<ImageState> for ImageStateOrLayout {
    fn from(state: ImageState) -> Self {
        ImageStateOrLayout::State(state)
    }
}

impl From<hal::image::Layout> for ImageStateOrLayout {
    fn from(layout: hal::image::Layout) -> Self {
        ImageStateOrLayout::Layout(layout)
    }
}

#[derive(Debug)]
pub(crate) struct Uploader<B: hal::Backend> {
    family_uploads: Vec<Option<parking_lot::Mutex<FamilyUploads<B>>>>,
}

impl<B> Uploader<B>
where
    B: hal::Backend,
{
    /// # Safety
    ///
    /// `families` must belong to the `device`
    pub(crate) unsafe fn new(
        device: &Device<B>,
        families: &Families<B>,
    ) -> Result<Self, OutOfMemory> {
        let mut family_uploads = Vec::new();
        for family in families.as_slice() {
            while family_uploads.len() <= family.id().index {
                family_uploads.push(None);
            }

            family_uploads[family.id().index] = Some(parking_lot::Mutex::new(FamilyUploads {
                fences: Vec::new(),
                pool: family
                    .create_pool(device)
                    .map(|pool| pool.with_capability().unwrap())?,
                next: Vec::new(),
                pending: VecDeque::new(),
                command_buffers: Vec::new(),
                barriers: Barriers::new(
                    hal::pso::PipelineStage::TRANSFER,
                    hal::buffer::Access::TRANSFER_WRITE,
                    hal::image::Access::TRANSFER_WRITE,
                ),
            }));
        }

        Ok(Uploader { family_uploads })
    }

    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Uploader`.
    /// `buffer` and `staging` must belong to the `device`.
    pub(crate) unsafe fn upload_buffer(
        &self,
        device: &Device<B>,
        buffer: &Buffer<B>,
        offset: u64,
        staging: Escape<Buffer<B>>,
        last: Option<BufferState>,
        next: BufferState,
    ) -> Result<(), OutOfMemory> {
        let mut family_uploads = self.family_uploads[next.queue.family.index]
            .as_ref()
            .unwrap()
            .lock();

        if let Some(last) = last {
            if last.queue != next.queue {
                unimplemented!("Can't sync resources across queues");
            }
        }

        family_uploads.barriers.add_buffer(
            last.map_or(hal::pso::PipelineStage::empty(), |l| l.stage),
            hal::buffer::Access::empty(),
            next.stage,
            next.access,
        );

        let next_upload = family_uploads.next_upload(device, next.queue.index)?;
        let mut encoder = next_upload.command_buffer.encoder();
        encoder.copy_buffer(
            &*staging,
            &*buffer,
            Some(hal::command::BufferCopy {
                src: 0,
                dst: offset,
                size: staging.size(),
            }),
        );

        next_upload.staging_buffers.push(staging);

        Ok(())
    }

    /// # Safety
    ///
    /// `image` must belong to the `device` that was used to create this Uploader.
    pub(crate) unsafe fn transition_image(
        &self,
        image: Handle<Image<B>>,
        image_range: hal::image::SubresourceRange,
        last: ImageStateOrLayout,
        next: ImageState,
    ) {
        use hal::image::{Access, Layout};

        let mut family_uploads = self.family_uploads[next.queue.family.index]
            .as_ref()
            .unwrap()
            .lock();

        let (last_stage, mut last_access, last_layout) = match last {
            ImageStateOrLayout::State(last) => {
                if last.queue != next.queue {
                    unimplemented!("Can't sync resources across queues");
                }
                (last.stage, last.access, last.layout)
            }
            ImageStateOrLayout::Layout(last_layout) => {
                (
                    hal::pso::PipelineStage::TOP_OF_PIPE,
                    Access::empty(),
                    last_layout,
                )
            }
        };

        if last_layout == Layout::Undefined || last_layout == next.layout {
            last_access = Access::empty();
        }

        family_uploads.barriers.add_image(
            image,
            image_range,
            last_stage,
            last_access,
            last_layout,
            next.layout,
            next.stage,
            next.access,
            next.layout,
        );
    }

    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Uploader`.
    /// `image` and `staging` must belong to the `device`.
    pub(crate) unsafe fn upload_image(
        &self,
        device: &Device<B>,
        image: Handle<Image<B>>,
        data_width: u32,
        data_height: u32,
        image_layers: hal::image::SubresourceLayers,
        image_offset: hal::image::Offset,
        image_extent: hal::image::Extent,
        staging: Escape<Buffer<B>>,
        last: ImageStateOrLayout,
        next: ImageState,
    ) -> Result<(), OutOfMemory> {
        use hal::image::{Access, Layout};

        let mut family_uploads = self.family_uploads[next.queue.family.index]
            .as_ref()
            .unwrap()
            .lock();

        let whole_extent = if image_layers.level == 0 {
            image.kind().extent()
        } else {
            image.kind().level_extent(image_layers.level)
        };

        let whole_level = image_offset == hal::image::Offset::ZERO && image_extent == whole_extent;

        let image_range = hal::image::SubresourceRange {
            aspects: image_layers.aspects,
            levels: image_layers.level..image_layers.level + 1,
            layers: image_layers.layers.clone(),
        };

        let (last_stage, mut last_access, last_layout) = match last {
            ImageStateOrLayout::State(last) => {
                if last.queue != next.queue {
                    unimplemented!("Can't sync resources across queues");
                }
                (
                    last.stage,
                    last.access,
                    if whole_level {
                        Layout::Undefined
                    } else {
                        last.layout
                    },
                )
            }
            ImageStateOrLayout::Layout(last_layout) => {
                (
                    hal::pso::PipelineStage::TOP_OF_PIPE,
                    Access::empty(),
                    if whole_level {
                        Layout::Undefined
                    } else {
                        last_layout
                    },
                )
            }
        };

        let target_layout = match (last_layout, next.layout) {
            (Layout::TransferDstOptimal, _) => Layout::TransferDstOptimal,
            (_, Layout::General) => Layout::General,
            (Layout::General, _) => Layout::General,
            _ => Layout::TransferDstOptimal,
        };

        if last_layout == Layout::Undefined || last_layout == target_layout {
            last_access = Access::empty();
        }

        family_uploads.barriers.add_image(
            image.clone(),
            image_range,
            last_stage,
            last_access,
            last_layout,
            target_layout,
            next.stage,
            next.access,
            next.layout,
        );

        let next_upload = family_uploads.next_upload(device, next.queue.index)?;
        let mut encoder = next_upload.command_buffer.encoder();
        encoder.copy_buffer_to_image(
            &*staging,
            image.raw(),
            target_layout,
            Some(hal::command::BufferImageCopy {
                buffer_offset: 0,
                buffer_width: data_width,
                buffer_height: data_height,
                image_layers,
                image_offset,
                image_extent,
            }),
        );

        next_upload.staging_buffers.push(staging);
        Ok(())
    }

    /// Cleanup pending updates.
    ///
    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Uploader`.
    pub(crate) unsafe fn cleanup(&mut self, device: &Device<B>) {
        for uploader in self.family_uploads.iter_mut() {
            if let Some(uploader) = uploader {
                uploader.get_mut().cleanup(device);
            }
        }
    }

    /// Flush new updates.
    ///
    /// # Safety
    ///
    /// `families` must be the same that was used to create this `Uploader`.
    pub(crate) unsafe fn flush(&mut self, families: &mut Families<B>) {
        for family in families.as_slice_mut() {
            let uploader = self.family_uploads[family.id().index]
                .as_mut()
                .expect("Uploader must be initialized for all families");
            uploader.get_mut().flush(family);
        }
    }

    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Uploader`.
    /// `device` must be idle.
    pub(crate) unsafe fn dispose(&mut self, device: &Device<B>) {
        self.family_uploads.drain(..).for_each(|fu| {
            if let Some(fu) = fu {
                fu.into_inner().dispose(device)
            }
        });
    }
}

#[derive(Debug)]
pub(crate) struct FamilyUploads<B: hal::Backend> {
    pool: CommandPool<B, Transfer, IndividualReset>,
    command_buffers:
        Vec<[CommandBuffer<B, Transfer, InitialState, PrimaryLevel, IndividualReset>; 2]>,
    next: Vec<Option<NextUploads<B>>>,
    pending: VecDeque<PendingUploads<B>>,
    fences: Vec<B::Fence>,
    barriers: Barriers<B>,
}

#[derive(Debug)]
pub(crate) struct PendingUploads<B: hal::Backend> {
    barrier_buffer: CommandBuffer<B, Transfer, PendingOnceState, PrimaryLevel, IndividualReset>,
    command_buffer: CommandBuffer<B, Transfer, PendingOnceState, PrimaryLevel, IndividualReset>,
    staging_buffers: Vec<Escape<Buffer<B>>>,
    fence: B::Fence,
}

#[derive(Debug)]
struct NextUploads<B: hal::Backend> {
    barrier_buffer:
        CommandBuffer<B, Transfer, RecordingState<OneShot>, PrimaryLevel, IndividualReset>,
    command_buffer:
        CommandBuffer<B, Transfer, RecordingState<OneShot>, PrimaryLevel, IndividualReset>,
    staging_buffers: Vec<Escape<Buffer<B>>>,
    fence: B::Fence,
}

impl<B> FamilyUploads<B>
where
    B: hal::Backend,
{
    unsafe fn flush(&mut self, family: &mut Family<B>) {
        for (queue, mut next) in self
            .next
            .drain(..)
            .enumerate()
            .filter_map(|(i, x)| x.map(|x| (i, x)))
        {
            let mut barriers_encoder = next.barrier_buffer.encoder();
            let mut encoder = next.command_buffer.encoder();

            self.barriers.encode_before(&mut barriers_encoder);
            self.barriers.encode_after(&mut encoder);

            let (barriers_submit, barrier_buffer) = next.barrier_buffer.finish().submit_once();
            let (submit, command_buffer) = next.command_buffer.finish().submit_once();

            family.queue_mut(queue).submit_raw_fence(
                Some(Submission::new().submits(once(barriers_submit).chain(once(submit)))),
                Some(&next.fence),
            );

            self.pending.push_back(PendingUploads {
                barrier_buffer,
                command_buffer,
                staging_buffers: next.staging_buffers,
                fence: next.fence,
            });
        }
    }

    unsafe fn next_upload(
        &mut self,
        device: &Device<B>,
        queue: usize,
    ) -> Result<&mut NextUploads<B>, OutOfMemory> {
        while self.next.len() <= queue {
            self.next.push(None);
        }

        let pool = &mut self.pool;

        match &mut self.next[queue] {
            Some(next) => Ok(next),
            slot @ None => {
                let [buf_a, buf_b] = self.command_buffers.pop().unwrap_or_else(|| {
                    let mut bufs = pool.allocate_buffers(2);
                    [bufs.remove(1), bufs.remove(0)]
                });
                let fence = self
                    .fences
                    .pop()
                    .map_or_else(|| device.create_fence(false), Ok)?;
                *slot = Some(NextUploads {
                    barrier_buffer: buf_a.begin(OneShot, ()),
                    command_buffer: buf_b.begin(OneShot, ()),
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
    unsafe fn cleanup(&mut self, device: &Device<B>) {
        while let Some(pending) = self.pending.pop_front() {
            match device.get_fence_status(&pending.fence) {
                Ok(false) => {
                    self.pending.push_front(pending);
                    return;
                }
                Err(hal::device::DeviceLost) => {
                    panic!("Device lost error is not handled yet");
                }
                Ok(true) => {
                    device
                        .reset_fence(&pending.fence)
                        .expect("Can always reset signalled fence");
                    self.fences.push(pending.fence);
                    self.command_buffers.push([
                        pending.command_buffer.mark_complete().reset(),
                        pending.barrier_buffer.mark_complete().reset(),
                    ]);
                }
            }
        }
    }

    /// # Safety
    ///
    /// Device must be idle.
    unsafe fn dispose(mut self, device: &Device<B>) {
        let pool = &mut self.pool;
        self.pending.drain(..).for_each(|pending| {
            device.destroy_fence(pending.fence);
            pool.free_buffers(Some(pending.command_buffer.mark_complete()));
            pool.free_buffers(Some(pending.barrier_buffer.mark_complete()));
        });

        self.fences
            .drain(..)
            .for_each(|fence| device.destroy_fence(fence));
        pool.free_buffers(
            self.command_buffers
                .drain(..)
                .flat_map(|[a, b]| once(a).chain(once(b))),
        );

        pool.free_buffers(self.next.drain(..).filter_map(|n| n).flat_map(|next| {
            device.destroy_fence(next.fence);
            once(next.command_buffer).chain(once(next.barrier_buffer))
        }));
        self.pool.dispose(device);
    }
}
