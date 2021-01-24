use std::{
    collections::VecDeque,
    iter::once,
    ops::{DerefMut, Range},
};

use hal::device::{Device as _, OutOfMemory};
use rendy_core::{hal, Device};
use smallvec::SmallVec;

use crate::{
    barriers::Barriers,
    command::{
        CommandBuffer, CommandPool, Encoder, Families, Family, Graphics, IndividualReset,
        InitialState, Level, OneShot, PendingOnceState, PrimaryLevel, QueueId, RecordingState,
        Submission, Supports,
    },
    resource::{Handle, Image},
    upload::ImageState,
};

/// Manages blitting images across families and queues.
#[derive(Debug)]
pub struct Blitter<B: hal::Backend> {
    family_ops: Vec<Option<parking_lot::Mutex<FamilyGraphicsOps<B>>>>,
}

fn subresource_to_range(sub: &hal::image::SubresourceLayers) -> hal::image::SubresourceRange {
    hal::image::SubresourceRange {
        aspects: sub.aspects,
        levels: sub.level..sub.level + 1,
        layers: sub.layers.clone(),
    }
}

/// A region to be blitted including the source and destination images and states,
#[derive(Debug, Clone)]
pub struct BlitRegion {
    /// Region to blit from
    pub src: BlitImageState,
    /// Region to blit to
    pub dst: BlitImageState,
}

impl BlitRegion {
    /// Get the blit regions needed to fill the mip levels of an image
    ///
    /// # Safety
    ///
    /// `last` state must be valid for corresponding image layer at the time of command execution (after memory transfers).
    /// `last` and `next` should contain at least `image.levels()` elements.
    /// `image.levels()` must be greater than 1
    pub fn mip_blits_for_image<B: hal::Backend>(
        image: &Handle<Image<B>>,
        last: impl IntoIterator<Item = ImageState>,
        next: impl IntoIterator<Item = ImageState>,
    ) -> (QueueId, Vec<BlitRegion>) {
        let aspects = image.format().surface_desc().aspects;

        let transfer = hal::pso::PipelineStage::TRANSFER;
        let src_optimal = hal::image::Layout::TransferSrcOptimal;
        let read = hal::image::Access::TRANSFER_READ;
        let write = hal::image::Access::TRANSFER_WRITE;

        let mut last_iter = last.into_iter();
        let mut next_iter = next.into_iter();

        let mut src_last = last_iter.next().unwrap();
        let mut src_next = next_iter.next().unwrap();

        let queue = src_last.queue;

        let mut blits = Vec::with_capacity(image.levels() as usize - 1);

        for (level, (dst_last, dst_next)) in (1..image.levels())
            .into_iter()
            .zip(last_iter.zip(next_iter))
        {
            let begin = level == 1;
            let end = level == image.levels() - 1;

            blits.push(BlitRegion {
                src: BlitImageState {
                    subresource: hal::image::SubresourceLayers {
                        aspects,
                        level: level - 1,
                        layers: 0..image.layers(),
                    },
                    bounds: hal::image::Offset::ZERO
                        .into_bounds(&image.kind().level_extent(level - 1)),
                    last_stage: if begin { src_last.stage } else { transfer },
                    last_access: if begin { src_last.access } else { write },
                    last_layout: if begin { src_last.layout } else { src_optimal },
                    next_stage: src_next.stage,
                    next_access: src_next.access,
                    next_layout: src_next.layout,
                },
                dst: BlitImageState {
                    subresource: hal::image::SubresourceLayers {
                        aspects,
                        level,
                        layers: 0..image.layers(),
                    },
                    bounds: hal::image::Offset::ZERO.into_bounds(&image.kind().level_extent(level)),
                    last_stage: dst_last.stage,
                    last_access: hal::image::Access::empty(),
                    last_layout: hal::image::Layout::Undefined,
                    next_stage: if end { dst_next.stage } else { transfer },
                    next_access: if end { dst_next.access } else { read },
                    next_layout: if end { dst_next.layout } else { src_optimal },
                },
            });

            src_last = dst_last;
            src_next = dst_next;
        }

        (queue, blits)
    }
}

impl From<BlitRegion> for hal::command::ImageBlit {
    fn from(blit: BlitRegion) -> Self {
        hal::command::ImageBlit {
            src_subresource: blit.src.subresource,
            src_bounds: blit.src.bounds,
            dst_subresource: blit.dst.subresource,
            dst_bounds: blit.dst.bounds,
        }
    }
}

/// A region and image states for one image in a blit.
#[derive(Debug, Clone)]
pub struct BlitImageState {
    /// Subresource to use for blit
    pub subresource: hal::image::SubresourceLayers,
    /// Image offset range to use for blit
    pub bounds: Range<hal::image::Offset>,
    /// Last image stage before blit
    pub last_stage: hal::pso::PipelineStage,
    /// Last image access before blit
    pub last_access: hal::image::Access,
    /// Last image layout before blit
    pub last_layout: hal::image::Layout,
    /// Image stage after blit
    pub next_stage: hal::pso::PipelineStage,
    /// Image access after blit
    pub next_access: hal::image::Access,
    /// Image layout after blit
    pub next_layout: hal::image::Layout,
}

impl<B> Blitter<B>
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
        let mut family_ops = Vec::new();
        for family in families.as_slice() {
            while family_ops.len() <= family.id().index {
                family_ops.push(None);
            }

            family_ops[family.id().index] = Some(parking_lot::Mutex::new(FamilyGraphicsOps {
                pool: family
                    .create_pool(device)
                    .map(|pool| pool.with_capability().unwrap())?,
                initial: Vec::new(),
                next: Vec::new(),
                pending: VecDeque::new(),
                read_barriers: Barriers::new(
                    hal::pso::PipelineStage::TRANSFER,
                    hal::buffer::Access::TRANSFER_READ,
                    hal::image::Access::TRANSFER_READ,
                ),
                write_barriers: Barriers::new(
                    hal::pso::PipelineStage::TRANSFER,
                    hal::buffer::Access::TRANSFER_WRITE,
                    hal::image::Access::TRANSFER_WRITE,
                ),
            }));
        }

        Ok(Blitter { family_ops })
    }
    /// Fill all mip levels from the first level of provided image.
    ///
    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Blitter`.
    /// `image` must belong to the `device`.
    /// `last` state must be valid for corresponding image layer at the time of command execution (after memory transfers).
    /// `last` and `next` should contain at least `image.levels()` elements.
    /// `image.levels()` must be greater than 1
    pub unsafe fn fill_mips(
        &self,
        device: &Device<B>,
        image: Handle<Image<B>>,
        filter: hal::image::Filter,
        last: impl IntoIterator<Item = ImageState>,
        next: impl IntoIterator<Item = ImageState>,
    ) -> Result<(), OutOfMemory> {
        let (queue, blits) = BlitRegion::mip_blits_for_image(&image, last, next);
        for blit in blits {
            self.blit_image(device, queue, &image, &image, filter, Some(blit))?;
        }
        Ok(())
    }

    /// Blit provided regions of `src_image` to `dst_image`.
    ///
    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Blitter`.
    /// `src` and `dst` must belong to the `device`.
    /// regions' `last_*` states must be valid at the time of command execution (after memory transfers).
    /// All regions must have distinct subresource layer and level combination.
    pub unsafe fn blit_image(
        &self,
        device: &Device<B>,
        queue_id: QueueId,
        src_image: &Handle<Image<B>>,
        dst_image: &Handle<Image<B>>,
        filter: hal::image::Filter,
        regions: impl IntoIterator<Item = BlitRegion>,
    ) -> Result<(), OutOfMemory> {
        let mut family_ops = self.family_ops[queue_id.family.index]
            .as_ref()
            .unwrap()
            .lock();

        family_ops.next_ops(device, queue_id.index)?;

        let FamilyGraphicsOps { next, .. } = family_ops.deref_mut();

        let next_ops = next[queue_id.index].as_mut().unwrap();
        let mut encoder = next_ops.command_buffer.encoder();

        blit_image(&mut encoder, src_image, dst_image, filter, regions);
        Ok(())
    }

    /// Cleanup pending updates.
    ///
    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Blitter`.
    pub(crate) unsafe fn cleanup(&mut self, device: &Device<B>) {
        for blitter in self.family_ops.iter_mut() {
            if let Some(blitter) = blitter {
                blitter.get_mut().cleanup(device);
            }
        }
    }

    /// Flush new updates.
    ///
    /// # Safety
    ///
    /// `families` must be the same that was used to create this `Blitter`.
    pub(crate) unsafe fn flush(&mut self, families: &mut Families<B>) {
        for family in families.as_slice_mut() {
            let blitter = self.family_ops[family.id().index]
                .as_mut()
                .expect("Blitter must be initialized for all families");
            blitter.get_mut().flush(family);
        }
    }

    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Blitter`.
    /// `device` must be idle.
    pub(crate) unsafe fn dispose(&mut self, device: &Device<B>) {
        self.family_ops.drain(..).for_each(|fu| {
            if let Some(fu) = fu {
                fu.into_inner().dispose(device)
            }
        });
    }
}

/// Blits one or more regions from src_image into dst_image using
/// specified Filter
///
/// # Safety
///
/// * `src_image` and `dst_image` must have been created from the same `Device`
/// as `encoder`
pub unsafe fn blit_image<B, C, L>(
    encoder: &mut Encoder<'_, B, C, L>,
    src_image: &Handle<Image<B>>,
    dst_image: &Handle<Image<B>>,
    filter: hal::image::Filter,
    regions: impl IntoIterator<Item = BlitRegion>,
) where
    B: hal::Backend,
    C: Supports<Graphics>,
    L: Level,
{
    let mut read_barriers = Barriers::new(
        hal::pso::PipelineStage::TRANSFER,
        hal::buffer::Access::TRANSFER_READ,
        hal::image::Access::TRANSFER_READ,
    );

    let mut write_barriers = Barriers::new(
        hal::pso::PipelineStage::TRANSFER,
        hal::buffer::Access::TRANSFER_WRITE,
        hal::image::Access::TRANSFER_WRITE,
    );

    let regions = regions
        .into_iter()
        .map(|reg| {
            read_barriers.add_image(
                src_image.clone(),
                subresource_to_range(&reg.src.subresource),
                reg.src.last_stage,
                reg.src.last_access,
                reg.src.last_layout,
                hal::image::Layout::TransferSrcOptimal,
                reg.src.next_stage,
                reg.src.next_access,
                reg.src.next_layout,
            );

            write_barriers.add_image(
                dst_image.clone(),
                subresource_to_range(&reg.dst.subresource),
                reg.dst.last_stage,
                reg.dst.last_access,
                reg.dst.last_layout,
                hal::image::Layout::TransferDstOptimal,
                reg.dst.next_stage,
                reg.dst.next_access,
                reg.dst.next_layout,
            );

            reg.into()
        })
        .collect::<SmallVec<[_; 1]>>();

    // TODO: synchronize whatever possible on flush.
    // Currently all barriers are inlined due to dependencies between blits.

    read_barriers.encode_before(encoder);
    write_barriers.encode_before(encoder);

    encoder.blit_image(
        src_image.raw(),
        hal::image::Layout::TransferSrcOptimal,
        dst_image.raw(),
        hal::image::Layout::TransferDstOptimal,
        filter,
        regions,
    );

    read_barriers.encode_after(encoder);
    write_barriers.encode_after(encoder);
}

#[derive(Debug)]
pub(crate) struct FamilyGraphicsOps<B: hal::Backend> {
    pool: CommandPool<B, Graphics, IndividualReset>,
    initial: Vec<GraphicsOps<B, InitialState>>,
    next: Vec<Option<GraphicsOps<B, RecordingState<OneShot>>>>,
    pending: VecDeque<GraphicsOps<B, PendingOnceState>>,
    read_barriers: Barriers<B>,
    write_barriers: Barriers<B>,
}

#[derive(Debug)]
struct GraphicsOps<B: hal::Backend, S> {
    command_buffer: CommandBuffer<B, Graphics, S, PrimaryLevel, IndividualReset>,
    fence: B::Fence,
}

impl<B> FamilyGraphicsOps<B>
where
    B: hal::Backend,
{
    unsafe fn flush(&mut self, family: &mut Family<B>) {
        for (queue, next) in self
            .next
            .drain(..)
            .enumerate()
            .filter_map(|(i, x)| x.map(|x| (i, x)))
        {
            let (submit, command_buffer) = next.command_buffer.finish().submit_once();

            family.queue_mut(queue).submit_raw_fence(
                Some(Submission::new().submits(once(submit))),
                Some(&next.fence),
            );

            self.pending.push_back(GraphicsOps {
                command_buffer,
                fence: next.fence,
            });
        }
    }

    unsafe fn next_ops(
        &mut self,
        device: &Device<B>,
        queue: usize,
    ) -> Result<&mut GraphicsOps<B, RecordingState<OneShot>>, OutOfMemory> {
        while self.next.len() <= queue {
            self.next.push(None);
        }

        let pool = &mut self.pool;

        match &mut self.next[queue] {
            Some(next) => Ok(next),
            slot @ None => {
                let initial: Result<_, OutOfMemory> = self.initial.pop().map_or_else(
                    || {
                        Ok(GraphicsOps {
                            command_buffer: pool.allocate_buffers(1).remove(0),
                            fence: device.create_fence(false)?,
                        })
                    },
                    Ok,
                );
                let initial = initial?;

                *slot = Some(GraphicsOps {
                    command_buffer: initial.command_buffer.begin(OneShot, ()),
                    fence: initial.fence,
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
                    self.initial.push(GraphicsOps {
                        command_buffer: pending.command_buffer.mark_complete().reset(),
                        fence: pending.fence,
                    })
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
            pool.free_buffers(once(pending.command_buffer.mark_complete()));
        });
        self.initial.drain(..).for_each(|initial| {
            device.destroy_fence(initial.fence);
            pool.free_buffers(once(initial.command_buffer));
        });
        self.next.drain(..).filter_map(|n| n).for_each(|next| {
            device.destroy_fence(next.fence);
            pool.free_buffers(once(next.command_buffer));
        });
        self.pool.dispose(device);
    }
}
