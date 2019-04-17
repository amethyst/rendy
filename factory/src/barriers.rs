use {
    crate::{
        command::Encoder,
        resource::{Handle, Image},
    },
    gfx_hal::{self, pso::PipelineStage, Backend},
    std::{
        iter::once,
        ops::{BitOrAssign, Range},
    },
};

#[derive(Debug)]
struct ImageBarrier<B: Backend> {
    /// The access flags controlling the image.
    pub states: Range<gfx_hal::image::State>,
    /// The image the barrier controls.
    pub target: Handle<Image<B>>,
    /// The source and destination Queue family IDs, for a [queue family ownership transfer](https://www.khronos.org/registry/vulkan/specs/1.0/html/vkspec.html#synchronization-queue-transfers)
    /// Can be `None` to indicate no ownership transfer.
    pub families: Option<Range<gfx_hal::queue::QueueFamilyId>>,
    /// A `SubresourceRange` that defines which section of an image the barrier applies to.
    pub range: gfx_hal::image::SubresourceRange,
}

impl<B: Backend> ImageBarrier<B> {
    fn raw(&self) -> gfx_hal::memory::Barrier<'_, B> {
        gfx_hal::memory::Barrier::Image {
            states: self.states.clone(),
            target: self.target.raw(),
            families: self.families.clone(),
            range: self.range.clone(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ImgBarrierCollector<B: Backend> {
    target_stage: PipelineStage,
    target_access: gfx_hal::image::Access,
    target_layout: gfx_hal::image::Layout,
    before_barriers: (Option<PipelineStage>, Vec<ImageBarrier<B>>),
    after_barriers: (Option<PipelineStage>, Vec<ImageBarrier<B>>),
    before_global: Option<(PipelineStage, gfx_hal::image::Access)>,
    after_global: Option<(PipelineStage, gfx_hal::image::Access)>,
}

impl<B: Backend> ImgBarrierCollector<B> {
    pub fn new(
        target_stage: PipelineStage,
        target_access: gfx_hal::image::Access,
        target_layout: gfx_hal::image::Layout,
    ) -> Self {
        Self {
            target_stage,
            target_access,
            target_layout,
            before_barriers: (None, Vec::new()),
            after_barriers: (None, Vec::new()),
            before_global: None,
            after_global: None,
        }
    }

    fn add_last_access(&mut self, stage: PipelineStage, access: gfx_hal::image::Access) {
        merge_opt(&mut self.before_global, (stage, access));
    }

    fn add_next_access(&mut self, stage: PipelineStage, access: gfx_hal::image::Access) {
        merge_opt(&mut self.after_global, (stage, access));
    }

    fn add_before(&mut self, stage: PipelineStage, barrier: ImageBarrier<B>) {
        let (mut_stage, vec) = &mut self.before_barriers;
        *mut_stage.get_or_insert(stage) |= stage;
        vec.push(barrier);
    }

    fn add_after(&mut self, stage: PipelineStage, barrier: ImageBarrier<B>) {
        let (mut_stage, vec) = &mut self.after_barriers;
        *mut_stage.get_or_insert(stage) |= stage;
        vec.push(barrier);
    }

    pub fn add_image(
        &mut self,
        image: Handle<Image<B>>,
        image_range: gfx_hal::image::SubresourceRange,
        last_stage: PipelineStage,
        last_access: gfx_hal::image::Access,
        last_layout: gfx_hal::image::Layout,
        next_stage: PipelineStage,
        next_access: gfx_hal::image::Access,
        next_layout: gfx_hal::image::Layout,
    ) {
        if last_layout == self.target_layout {
            self.add_last_access(last_stage, last_access);
        } else {
            self.add_before(
                last_stage,
                ImageBarrier {
                    states: (last_access, last_layout)..(self.target_access, self.target_layout),
                    target: image.clone(),
                    families: None,
                    range: image_range.clone(),
                },
            );
        }

        if next_layout == self.target_layout {
            self.add_next_access(next_stage, next_access);
        } else {
            self.add_after(
                next_stage,
                ImageBarrier {
                    states: (self.target_access, self.target_layout)..(last_access, last_layout),
                    target: image,
                    families: None,
                    range: image_range,
                },
            )
        }
    }

    pub fn encode_before<C, L>(&mut self, encoder: &mut Encoder<'_, B, C, L>) {
        let barriers_iter = self.before_barriers.1.iter().map(|b| b.raw());

        if let Some((mut stage, access)) = self.before_global.take() {
            if let Some(stage2) = self.before_barriers.0.take() {
                stage |= stage2;
            }
            encoder.pipeline_barrier(
                stage..self.target_stage,
                gfx_hal::memory::Dependencies::empty(),
                once(gfx_hal::memory::Barrier::AllImages(
                    access..self.target_access,
                ))
                .chain(barriers_iter),
            );
        } else if let Some(stage) = self.before_barriers.0.take() {
            encoder.pipeline_barrier(
                stage..self.target_stage,
                gfx_hal::memory::Dependencies::empty(),
                barriers_iter,
            );
        }
        self.before_barriers.1.clear();
    }

    pub fn encode_after<C, L>(&mut self, encoder: &mut Encoder<'_, B, C, L>) {
        let barriers_iter = self.after_barriers.1.iter().map(|b| b.raw());

        if let Some((mut stage, access)) = self.after_global.take() {
            if let Some(stage2) = self.after_barriers.0.take() {
                stage |= stage2;
            }
            encoder.pipeline_barrier(
                self.target_stage..stage,
                gfx_hal::memory::Dependencies::empty(),
                once(gfx_hal::memory::Barrier::AllImages(
                    self.target_access..access,
                ))
                .chain(barriers_iter),
            );
        } else if let Some(stage) = self.after_barriers.0.take() {
            encoder.pipeline_barrier(
                self.target_stage..stage,
                gfx_hal::memory::Dependencies::empty(),
                barriers_iter,
            );
        }
        self.after_barriers.1.clear();
    }
}

#[derive(Debug)]
pub(crate) struct BufBarrierCollector {
    target_stage: PipelineStage,
    target_access: gfx_hal::buffer::Access,
    before_global: Option<(PipelineStage, gfx_hal::buffer::Access)>,
    after_global: Option<(PipelineStage, gfx_hal::buffer::Access)>,
}

impl BufBarrierCollector {
    pub fn new(target_stage: PipelineStage, target_access: gfx_hal::buffer::Access) -> Self {
        Self {
            target_stage,
            target_access,
            before_global: None,
            after_global: None,
        }
    }

    pub fn add_buffer(
        &mut self,
        last: Option<(PipelineStage, gfx_hal::buffer::Access)>,
        next: (PipelineStage, gfx_hal::buffer::Access),
    ) {
        if let Some(last) = last {
            if last.1 != self.target_access {
                merge_opt(&mut self.before_global, last);
            }
        }
        if next.1 != self.target_access {
            merge_opt(&mut self.after_global, next);
        }
    }

    pub fn encode_before<B: Backend, C, L>(&mut self, encoder: &mut Encoder<'_, B, C, L>) {
        if let Some((mut stage, access)) = self.after_global.take() {
            encoder.pipeline_barrier(
                stage..self.target_stage,
                gfx_hal::memory::Dependencies::empty(),
                once(gfx_hal::memory::Barrier::AllBuffers(
                    access..self.target_access,
                )),
            );
        }
    }

    pub fn encode_after<B: Backend, C, L>(&mut self, encoder: &mut Encoder<'_, B, C, L>) {
        if let Some((mut stage, access)) = self.after_global.take() {
            encoder.pipeline_barrier(
                self.target_stage..stage,
                gfx_hal::memory::Dependencies::empty(),
                once(gfx_hal::memory::Barrier::AllBuffers(
                    self.target_access..access,
                )),
            );
        }
    }
}

fn merge_opt<A: BitOrAssign, B: BitOrAssign>(opt: &mut Option<(A, B)>, with: (A, B)) {
    if let Some((a, b)) = opt {
        *a |= with.0;
        *b |= with.1;
    } else {
        opt.replace(with);
    }
}
