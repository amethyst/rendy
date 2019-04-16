use std::{collections::VecDeque, iter::once, ops::BitOrAssign};

use {
    crate::{
        command::{
            CommandBuffer, CommandPool, Families, Family, IndividualReset, InitialState, OneShot,
            PendingOnceState, PrimaryLevel, QueueId, RecordingState, Submission, Transfer,
        },
        resource::{Buffer, Escape, Image},
        util::Device,
    },
    gfx_hal::pso::PipelineStage,
    gfx_hal::Device as _,
};

/// State of the buffer on device.
#[derive(Clone, Copy, Debug)]
pub struct BufferState {
    /// Queue that uses the buffer.
    pub queue: QueueId,

    /// Stages when buffer get used.
    pub stage: PipelineStage,

    /// Access performed by device.
    pub access: gfx_hal::buffer::Access,
}

impl BufferState {
    /// Create default buffet state.
    pub fn new(queue: QueueId) -> Self {
        BufferState {
            queue,
            stage: PipelineStage::TOP_OF_PIPE,
            access: gfx_hal::buffer::Access::all(),
        }
    }

    /// Set specific stage.
    pub fn with_stage(mut self, stage: PipelineStage) -> Self {
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
    pub stage: PipelineStage,

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
            stage: PipelineStage::TOP_OF_PIPE,
            access: gfx_hal::image::Access::all(),
            layout,
        }
    }

    /// Set specific stage.
    pub fn with_stage(mut self, stage: PipelineStage) -> Self {
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
    family_uploads: Vec<Option<parking_lot::Mutex<FamilyUploads<B>>>>,
}

impl<B> Uploader<B>
where
    B: gfx_hal::Backend,
{
    /// # Safety
    ///
    /// `families` must belong to the `device`
    pub(crate) unsafe fn new(
        device: &Device<B>,
        families: &Families<B>,
    ) -> Result<Self, gfx_hal::device::OutOfMemory> {
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
                barriers_buffers: Vec::new(),
                before_barriers: (Vec::new(), None),
                after_barriers: (Vec::new(), None),
                before_global_buf: None,
                before_global_img: None,
                after_global_buf: None,
                after_global_img: None,
            }));
        }

        Ok(Uploader { family_uploads })
    }

    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Uploader`.
    /// `buffer` and `staging` must belong to the `device`.
    ///
    pub(crate) unsafe fn upload_buffer(
        &self,
        device: &Device<B>,
        buffer: &Buffer<B>,
        offset: u64,
        staging: Escape<Buffer<B>>,
        last: Option<BufferState>,
        next: BufferState,
    ) -> Result<(), failure::Error> {
        let mut family_uploads = self.family_uploads[next.queue.family.index]
            .as_ref()
            .unwrap()
            .lock();

        if let Some(last) = last {
            if last.queue != next.queue {
                unimplemented!("Can't sync resources across queues");
            }
            family_uploads.add_last_buf_access(last.stage, last.access);
        }
        family_uploads.add_next_buf_access(next.stage, next.access);

        let next_upload = family_uploads.next_upload(device, next.queue.index)?;
        let mut encoder = next_upload.command_buffer.encoder();
        encoder.copy_buffer(
            staging.raw(),
            buffer.raw(),
            Some(gfx_hal::command::BufferCopy {
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
    /// `device` must be the same that was used to create this `Uploader`.
    /// `image` and `staging` must belong to the `device`.
    ///
    pub(crate) unsafe fn upload_image(
        &self,
        device: &Device<B>,
        image: &Image<B>,
        data_width: u32,
        data_height: u32,
        image_layers: gfx_hal::image::SubresourceLayers,
        image_offset: gfx_hal::image::Offset,
        image_extent: gfx_hal::image::Extent,
        staging: Escape<Buffer<B>>,
        last: ImageStateOrLayout,
        next: ImageState,
    ) -> Result<(), failure::Error> {
        let mut family_uploads = self.family_uploads[next.queue.family.index]
            .as_ref()
            .unwrap()
            .lock();

        let whole_image =
            image_offset == gfx_hal::image::Offset::ZERO && image_extent == image.kind().extent();

        let image_range = gfx_hal::image::SubresourceRange {
            aspects: image_layers.aspects,
            levels: image_layers.level..image_layers.level + 1,
            layers: image_layers.layers.clone(),
        };

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

                if last_layout == gfx_hal::image::Layout::TransferDstOptimal {
                    family_uploads.add_last_img_access(last.stage, last.access);
                } else {
                    family_uploads.add_before_barrier(
                        last.stage,
                        gfx_hal::memory::Barrier::Image {
                            states: (last.access, last_layout)
                                ..(
                                    gfx_hal::image::Access::TRANSFER_WRITE,
                                    gfx_hal::image::Layout::TransferDstOptimal,
                                ),
                            target: std::mem::transmute(image.raw()),
                            families: None,
                            range: image_range.clone(),
                        },
                    );
                }
            }
            ImageStateOrLayout::Layout(mut last_layout) => {
                if last_layout != gfx_hal::image::Layout::TransferDstOptimal {
                    if whole_image {
                        last_layout = gfx_hal::image::Layout::Undefined;
                    }

                    family_uploads.add_before_barrier(
                        PipelineStage::TOP_OF_PIPE,
                        gfx_hal::memory::Barrier::Image {
                            states: (gfx_hal::image::Access::empty(), last_layout)
                                ..(
                                    gfx_hal::image::Access::TRANSFER_WRITE,
                                    gfx_hal::image::Layout::TransferDstOptimal,
                                ),
                            target: std::mem::transmute(image.raw()),
                            families: None,
                            range: image_range.clone(),
                        },
                    );
                }
            }
        }

        if next.layout == gfx_hal::image::Layout::TransferDstOptimal {
            family_uploads.add_next_img_access(next.stage, next.access);
        } else {
            family_uploads.add_after_barrier(
                next.stage,
                gfx_hal::memory::Barrier::Image {
                    states: (
                        gfx_hal::image::Access::TRANSFER_WRITE,
                        gfx_hal::image::Layout::TransferDstOptimal,
                    )..(next.access, next.layout),
                    target: std::mem::transmute(image.raw()),
                    families: None,
                    range: image_range.clone(),
                },
            )
        }

        let next_upload = family_uploads.next_upload(device, next.queue.index)?;
        let mut encoder = next_upload.command_buffer.encoder();
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
    ///
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
    ///
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
    ///
    pub(crate) unsafe fn dispose(&mut self, device: &Device<B>) {
        self.family_uploads.drain(..).for_each(|fu| {
            fu.map(|fu| fu.into_inner().dispose(device));
        });
    }
}

#[derive(Debug)]
pub(crate) struct FamilyUploads<B: gfx_hal::Backend> {
    pool: CommandPool<B, Transfer, IndividualReset>,
    barriers_buffers: Vec<CommandBuffer<B, Transfer, InitialState, PrimaryLevel, IndividualReset>>,
    command_buffers: Vec<CommandBuffer<B, Transfer, InitialState, PrimaryLevel, IndividualReset>>,
    next: Vec<Option<NextUploads<B>>>,
    pending: VecDeque<PendingUploads<B>>,
    fences: Vec<B::Fence>,
    before_barriers: (
        Vec<gfx_hal::memory::Barrier<'static, B>>,
        Option<PipelineStage>,
    ),
    after_barriers: (
        Vec<gfx_hal::memory::Barrier<'static, B>>,
        Option<PipelineStage>,
    ),
    before_global_buf: Option<(gfx_hal::buffer::Access, PipelineStage)>,
    before_global_img: Option<(gfx_hal::image::Access, PipelineStage)>,
    after_global_buf: Option<(gfx_hal::buffer::Access, PipelineStage)>,
    after_global_img: Option<(gfx_hal::image::Access, PipelineStage)>,
}

fn merge_access<A: BitOrAssign, B: BitOrAssign>(opt: &mut Option<(A, B)>, with: (A, B)) {
    if let Some((a, b)) = opt {
        *a |= with.0;
        *b |= with.1;
    } else {
        opt.replace(with);
    }
}

#[derive(Debug)]
pub(crate) struct PendingUploads<B: gfx_hal::Backend> {
    barriers_buffer: CommandBuffer<B, Transfer, PendingOnceState, PrimaryLevel, IndividualReset>,
    command_buffer: CommandBuffer<B, Transfer, PendingOnceState, PrimaryLevel, IndividualReset>,
    staging_buffers: Vec<Escape<Buffer<B>>>,
    fence: B::Fence,
}

#[derive(Debug)]
struct NextUploads<B: gfx_hal::Backend> {
    barriers_buffer:
        CommandBuffer<B, Transfer, RecordingState<OneShot>, PrimaryLevel, IndividualReset>,
    command_buffer:
        CommandBuffer<B, Transfer, RecordingState<OneShot>, PrimaryLevel, IndividualReset>,
    staging_buffers: Vec<Escape<Buffer<B>>>,
    fence: B::Fence,
}

impl<B> FamilyUploads<B>
where
    B: gfx_hal::Backend,
{
    fn add_last_img_access(&mut self, stage: PipelineStage, access: gfx_hal::image::Access) {
        merge_access(&mut self.before_global_img, (access, stage));
    }

    fn add_next_img_access(&mut self, stage: PipelineStage, access: gfx_hal::image::Access) {
        merge_access(&mut self.after_global_img, (access, stage));
    }

    fn add_last_buf_access(&mut self, stage: PipelineStage, access: gfx_hal::buffer::Access) {
        merge_access(&mut self.before_global_buf, (access, stage));
    }

    fn add_next_buf_access(&mut self, stage: PipelineStage, access: gfx_hal::buffer::Access) {
        merge_access(&mut self.after_global_buf, (access, stage));
    }

    fn add_before_barrier(
        &mut self,
        stage: PipelineStage,
        barrier: gfx_hal::memory::Barrier<'static, B>,
    ) {
        let (vec, mut_stage) = &mut self.before_barriers;
        *mut_stage.get_or_insert(stage) |= stage;
        vec.push(barrier);
    }

    fn add_after_barrier(
        &mut self,
        stage: PipelineStage,
        barrier: gfx_hal::memory::Barrier<'static, B>,
    ) {
        let (vec, mut_stage) = &mut self.after_barriers;
        *mut_stage.get_or_insert(stage) |= stage;
        vec.push(barrier);
    }

    unsafe fn flush(&mut self, family: &mut Family<B>) {
        for (queue, mut next) in self
            .next
            .drain(..)
            .enumerate()
            .filter_map(|(i, x)| x.map(|x| (i, x)))
        {
            let mut barriers_encoder = next.barriers_buffer.encoder();
            let mut encoder = next.command_buffer.encoder();

            if let Some((access, stage)) = self.before_global_img.take() {
                barriers_encoder.pipeline_barrier(
                    stage..PipelineStage::TRANSFER,
                    gfx_hal::memory::Dependencies::empty(),
                    Some(gfx_hal::memory::Barrier::AllImages(
                        access..gfx_hal::image::Access::TRANSFER_WRITE,
                    )),
                );
            }
            if let Some((access, stage)) = self.before_global_buf.take() {
                barriers_encoder.pipeline_barrier(
                    stage..PipelineStage::TRANSFER,
                    gfx_hal::memory::Dependencies::empty(),
                    Some(gfx_hal::memory::Barrier::AllBuffers(
                        access..gfx_hal::buffer::Access::TRANSFER_WRITE,
                    )),
                );
            }
            if let Some(stage) = self.before_barriers.1.take() {
                barriers_encoder.pipeline_barrier(
                    stage..PipelineStage::TRANSFER,
                    gfx_hal::memory::Dependencies::empty(),
                    self.after_barriers.0.drain(..),
                );
            }
            if let Some((access, stage)) = self.after_global_img.take() {
                encoder.pipeline_barrier(
                    stage..PipelineStage::TRANSFER,
                    gfx_hal::memory::Dependencies::empty(),
                    Some(gfx_hal::memory::Barrier::AllImages(
                        gfx_hal::image::Access::TRANSFER_WRITE..access,
                    )),
                );
            }
            if let Some((access, stage)) = self.after_global_buf.take() {
                encoder.pipeline_barrier(
                    PipelineStage::TRANSFER..stage,
                    gfx_hal::memory::Dependencies::empty(),
                    Some(gfx_hal::memory::Barrier::AllBuffers(
                        gfx_hal::buffer::Access::TRANSFER_WRITE..access,
                    )),
                );
            }
            if let Some(stage) = self.after_barriers.1.take() {
                encoder.pipeline_barrier(
                    PipelineStage::TRANSFER..stage,
                    gfx_hal::memory::Dependencies::empty(),
                    self.after_barriers.0.drain(..),
                );
            }

            let (barriers_submit, barriers_buffer) = next.barriers_buffer.finish().submit_once();
            let (submit, command_buffer) = next.command_buffer.finish().submit_once();

            family.queue_mut(queue).submit_raw_fence(
                Some(Submission::new().submits(once(barriers_submit).chain(once(submit)))),
                Some(&next.fence),
            );

            self.pending.push_back(PendingUploads {
                barriers_buffer,
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
    ) -> Result<&mut NextUploads<B>, failure::Error> {
        while self.next.len() <= queue {
            self.next.push(None);
        }

        let pool = &mut self.pool;

        match &mut self.next[queue] {
            Some(next) => Ok(next),
            slot @ None => {
                let command_buffer = self
                    .command_buffers
                    .pop()
                    .unwrap_or_else(|| pool.allocate_buffers(1).pop().unwrap());
                let barriers_buffer = self
                    .barriers_buffers
                    .pop()
                    .unwrap_or_else(|| pool.allocate_buffers(1).pop().unwrap());

                let fence = self
                    .fences
                    .pop()
                    .map_or_else(|| device.create_fence(false), Ok)?;
                *slot = Some(NextUploads {
                    barriers_buffer: barriers_buffer.begin(OneShot, ()),
                    command_buffer: command_buffer.begin(OneShot, ()),
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
    unsafe fn cleanup(&mut self, device: &Device<B>) {
        while let Some(pending) = self.pending.pop_front() {
            match device.get_fence_status(&pending.fence) {
                Ok(false) => {
                    self.pending.push_front(pending);
                    return;
                }
                Err(gfx_hal::device::DeviceLost) => {
                    panic!("Device lost error is not handled yet");
                }
                Ok(true) => {
                    self.fences.push(pending.fence);
                    self.command_buffers
                        .push(pending.command_buffer.mark_complete().reset());
                    self.barriers_buffers
                        .push(pending.barriers_buffer.mark_complete().reset());
                }
            }
        }
    }

    /// # Safety
    ///
    /// Device must be idle.
    ///
    unsafe fn dispose(mut self, device: &Device<B>) {
        let pool = &mut self.pool;
        self.pending.drain(..).for_each(|pending| {
            device.destroy_fence(pending.fence);
            pool.free_buffers(Some(pending.command_buffer.mark_complete()))
        });

        self.fences
            .drain(..)
            .for_each(|fence| device.destroy_fence(fence));
        pool.free_buffers(self.command_buffers.drain(..));
        pool.free_buffers(self.barriers_buffers.drain(..));
        pool.free_buffers(self.next.drain(..).filter_map(|n| n).flat_map(|next| {
            device.destroy_fence(next.fence);
            once(next.command_buffer).chain(once(next.barriers_buffer))
        }));
        drop(pool);
        self.pool.dispose(device);
    }
}
