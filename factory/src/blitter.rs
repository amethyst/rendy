use {
    crate::{
        barriers::Barriers,
        command::{
            CommandBuffer, CommandPool, Families, Family, Graphics, IndividualReset, InitialState,
            OneShot, PendingOnceState, PrimaryLevel, QueueId, RecordingState, Submission,
        },
        resource::{Handle, Image},
        upload::ImageState,
        util::Device,
    },
    gfx_hal::Device as _,
    smallvec::SmallVec,
    std::{collections::VecDeque, iter::once, ops::DerefMut, ops::Range},
};

#[derive(Debug)]
pub struct Blitter<B: gfx_hal::Backend> {
    family_ops: Vec<Option<parking_lot::Mutex<FamilyGraphicsOps<B>>>>,
}

fn subresource_to_range(
    sub: &gfx_hal::image::SubresourceLayers,
) -> gfx_hal::image::SubresourceRange {
    gfx_hal::image::SubresourceRange {
        aspects: sub.aspects,
        levels: sub.level..sub.level + 1,
        layers: sub.layers.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct BlitRegion {
    pub src: BlitImageState,
    pub dst: BlitImageState,
}

#[derive(Debug, Clone)]
pub struct BlitImageState {
    subresource: gfx_hal::image::SubresourceLayers,
    bounds: Range<gfx_hal::image::Offset>,
    last_stage: gfx_hal::pso::PipelineStage,
    last_access: gfx_hal::image::Access,
    last_layout: gfx_hal::image::Layout,
    next_stage: gfx_hal::pso::PipelineStage,
    next_access: gfx_hal::image::Access,
    next_layout: gfx_hal::image::Layout,
}

impl<B> Blitter<B>
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
                    gfx_hal::pso::PipelineStage::TRANSFER,
                    gfx_hal::buffer::Access::TRANSFER_READ,
                    gfx_hal::image::Access::TRANSFER_READ,
                ),
                write_barriers: Barriers::new(
                    gfx_hal::pso::PipelineStage::TRANSFER,
                    gfx_hal::buffer::Access::TRANSFER_WRITE,
                    gfx_hal::image::Access::TRANSFER_WRITE,
                ),
            }));
        }

        Ok(Blitter { family_ops })
    }
    /// `dst` should be `None` when blitting from the same image.
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
        filter: gfx_hal::image::Filter,
        last: impl IntoIterator<Item = ImageState>,
        next: impl IntoIterator<Item = ImageState>,
    ) -> Result<(), failure::Error> {
        assert!(image.levels() > 1);

        let aspects = image.format().surface_desc().aspects;

        let transfer = gfx_hal::pso::PipelineStage::TRANSFER;
        let src_optimal = gfx_hal::image::Layout::TransferSrcOptimal;
        let read = gfx_hal::image::Access::TRANSFER_READ;
        let write = gfx_hal::image::Access::TRANSFER_WRITE;

        let mut last_iter = last.into_iter();
        let mut next_iter = next.into_iter();

        let mut src_last = last_iter.next().unwrap();
        let mut src_next = next_iter.next().unwrap();
        assert_eq!(src_last.queue, src_next.queue);

        for (level, (dst_last, dst_next)) in (1..image.levels())
            .into_iter()
            .zip(last_iter.zip(next_iter))
        {
            assert_eq!(dst_last.queue, dst_next.queue);

            let begin = level == 1;
            let end = level == image.levels() - 1;

            let blit = BlitRegion {
                src: BlitImageState {
                    subresource: gfx_hal::image::SubresourceLayers {
                        aspects,
                        level: level - 1,
                        layers: 0..image.layers(),
                    },
                    bounds: gfx_hal::image::Offset::ZERO
                        .into_bounds(&image.kind().level_extent(level - 1)),
                    last_stage: if begin { src_last.stage } else { transfer },
                    last_access: if begin { src_last.access } else { write },
                    last_layout: if begin { src_last.layout } else { src_optimal },
                    next_stage: src_next.stage,
                    next_access: src_next.access,
                    next_layout: src_next.layout,
                },
                dst: BlitImageState {
                    subresource: gfx_hal::image::SubresourceLayers {
                        aspects,
                        level,
                        layers: 0..image.layers(),
                    },
                    bounds: gfx_hal::image::Offset::ZERO
                        .into_bounds(&image.kind().level_extent(level)),
                    last_stage: dst_last.stage,
                    last_access: gfx_hal::image::Access::empty(),
                    last_layout: gfx_hal::image::Layout::Undefined,
                    next_stage: if end { dst_next.stage } else { transfer },
                    next_access: if end { dst_next.access } else { read },
                    next_layout: if end { dst_next.layout } else { src_optimal },
                },
            };

            log::trace!("Blit: {:#?}", blit);
            self.blit_image(device, src_last.queue, &image, &image, filter, Some(blit))?;
            src_last = dst_last;
            src_next = dst_next;
        }
        Ok(())
    }

    /// `dst` should be `None` when blitting from the same image.
    ///
    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Blitter`.
    /// `src` and `dst` must belong to the `device`.
    /// regions' `last_*` states must be valid at the time of command execution (after memory transfers).
    ///
    pub unsafe fn blit_image(
        &self,
        device: &Device<B>,
        queue_id: QueueId,
        src_image: &Handle<Image<B>>,
        dst_image: &Handle<Image<B>>,
        filter: gfx_hal::image::Filter,
        regions: impl IntoIterator<Item = BlitRegion>,
    ) -> Result<(), failure::Error> {
        let mut family_ops = self.family_ops[queue_id.family.index]
            .as_ref()
            .unwrap()
            .lock();

        family_ops.next_ops(device, queue_id.index)?;

        let FamilyGraphicsOps {
            next,
            read_barriers,
            write_barriers,
            ..
        } = family_ops.deref_mut();

        let next_ops = next[queue_id.index].as_mut().unwrap();
        let mut encoder = next_ops.command_buffer.encoder();

        let regions = regions
            .into_iter()
            .map(|reg| {
                read_barriers.add_image(
                    src_image.clone(),
                    subresource_to_range(&reg.src.subresource),
                    reg.src.last_stage,
                    reg.src.last_access,
                    reg.src.last_layout,
                    gfx_hal::image::Layout::TransferSrcOptimal,
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
                    gfx_hal::image::Layout::TransferDstOptimal,
                    reg.dst.next_stage,
                    reg.dst.next_access,
                    reg.dst.next_layout,
                );

                gfx_hal::command::ImageBlit {
                    src_subresource: reg.src.subresource,
                    src_bounds: reg.src.bounds,
                    dst_subresource: reg.dst.subresource,
                    dst_bounds: reg.dst.bounds,
                }
            })
            .collect::<SmallVec<[_; 1]>>();

        // TODO: synchronize whatever possible on flush.
        // Currently all barriers are inlined due to dependencies between blits.

        read_barriers.encode_before(&mut encoder);
        write_barriers.encode_before(&mut encoder);

        encoder.blit_image(
            src_image.raw(),
            gfx_hal::image::Layout::TransferSrcOptimal,
            dst_image.raw(),
            gfx_hal::image::Layout::TransferDstOptimal,
            filter,
            regions,
        );

        read_barriers.encode_after(&mut encoder);
        write_barriers.encode_after(&mut encoder);
        Ok(())
    }

    /// Cleanup pending updates.
    ///
    /// # Safety
    ///
    /// `device` must be the same that was used to create this `Blitter`.
    ///
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
    ///
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
    ///
    pub(crate) unsafe fn dispose(&mut self, device: &Device<B>) {
        self.family_ops.drain(..).for_each(|fu| {
            fu.map(|fu| fu.into_inner().dispose(device));
        });
    }
}

#[derive(Debug)]
pub(crate) struct FamilyGraphicsOps<B: gfx_hal::Backend> {
    pool: CommandPool<B, Graphics, IndividualReset>,
    initial: Vec<GraphicsOps<B, InitialState>>,
    next: Vec<Option<GraphicsOps<B, RecordingState<OneShot>>>>,
    pending: VecDeque<GraphicsOps<B, PendingOnceState>>,
    read_barriers: Barriers<B>,
    write_barriers: Barriers<B>,
}

#[derive(Debug)]
struct GraphicsOps<B: gfx_hal::Backend, S> {
    command_buffer: CommandBuffer<B, Graphics, S, PrimaryLevel, IndividualReset>,
    fence: B::Fence,
}

impl<B> FamilyGraphicsOps<B>
where
    B: gfx_hal::Backend,
{
    unsafe fn flush(&mut self, family: &mut Family<B>) {
        for (queue, next) in self
            .next
            .drain(..)
            .enumerate()
            .filter_map(|(i, x)| x.map(|x| (i, x)))
        {
            log::trace!("Flush blitter");
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
    ) -> Result<&mut GraphicsOps<B, RecordingState<OneShot>>, failure::Error> {
        while self.next.len() <= queue {
            self.next.push(None);
        }

        let pool = &mut self.pool;

        match &mut self.next[queue] {
            Some(next) => Ok(next),
            slot @ None => {
                let initial: Result<_, failure::Error> = self.initial.pop().map_or_else(
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
                Ok(true) => self.initial.push(GraphicsOps {
                    command_buffer: pending.command_buffer.mark_complete().reset(),
                    fence: pending.fence,
                }),
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
        drop(pool);
        self.pool.dispose(device);
    }
}
