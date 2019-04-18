use {
    crate::{
        command::Encoder,
        resource::{Handle, Image},
    },
    gfx_hal::{self, buffer, image, pso, Backend},
    std::{iter::once, ops::Range},
};

/// A variant of `gfx_hal::image::Barrier` that uses Handle<Image<B>>
#[derive(Debug)]
struct ImageBarrier<B: Backend> {
    /// The access flags controlling the image.
    pub states: Range<image::State>,
    /// The image the barrier controls.
    pub target: Handle<Image<B>>,
    /// A `SubresourceRange` that defines which section of an image the barrier applies to.
    pub range: image::SubresourceRange,
    // TODO: support queue transfers
    // pub families: Option<Range<gfx_hal::queue::QueueFamilyId>>,
}

impl<B: Backend> ImageBarrier<B> {
    fn raw(&self) -> gfx_hal::memory::Barrier<'_, B> {
        gfx_hal::memory::Barrier::Image {
            states: self.states.clone(),
            target: self.target.raw(),
            families: None,
            range: self.range.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Barriers<B: Backend> {
    before_stages: pso::PipelineStage,
    before_buffer_access: buffer::Access,
    before_image_access: image::Access,
    before_image_transitions: Vec<ImageBarrier<B>>,
    target_stages: pso::PipelineStage,
    target_buffer_access: buffer::Access,
    target_image_access: image::Access,
    after_stages: pso::PipelineStage,
    after_buffer_access: buffer::Access,
    after_image_access: image::Access,
    after_image_transitions: Vec<ImageBarrier<B>>,
}

impl<B: Backend> Barriers<B> {
    pub fn new(
        target_stages: pso::PipelineStage,
        target_buffer_access: buffer::Access,
        target_image_access: image::Access,
    ) -> Self {
        Self {
            before_stages: pso::PipelineStage::empty(),
            before_buffer_access: buffer::Access::empty(),
            before_image_access: image::Access::empty(),
            before_image_transitions: Vec::new(),
            target_stages,
            target_buffer_access,
            target_image_access,
            after_stages: pso::PipelineStage::empty(),
            after_buffer_access: buffer::Access::empty(),
            after_image_access: image::Access::empty(),
            after_image_transitions: Vec::new(),
        }
    }

    pub fn add_image(
        &mut self,
        image: Handle<Image<B>>,
        image_range: gfx_hal::image::SubresourceRange,
        last_stage: pso::PipelineStage,
        last_access: gfx_hal::image::Access,
        last_layout: gfx_hal::image::Layout,
        target_layout: image::Layout,
        next_stage: pso::PipelineStage,
        next_access: gfx_hal::image::Access,
        next_layout: gfx_hal::image::Layout,
    ) {
        log::trace!("Add image");
        if last_layout != target_layout {
            self.before_stages |= last_stage;
            self.before_image_access |= last_access;
            log::trace!(
                "Transition last: {:?}",
                (last_access, last_layout)..(self.target_image_access, target_layout)
            );
            self.before_image_transitions.push(ImageBarrier {
                states: (last_access, last_layout)..(self.target_image_access, target_layout),
                target: image.clone(),
                range: image_range.clone(),
            });
        } else if !last_access.contains(self.target_image_access) {
            self.before_stages |= last_stage;
            self.before_image_access |= last_access;
        }

        if next_layout != target_layout {
            self.after_stages |= next_stage;
            self.after_image_access |= next_access;
            log::trace!(
                "Transition next: {:?}",
                (self.target_image_access, target_layout)..(next_access, next_layout)
            );
            self.after_image_transitions.push(ImageBarrier {
                states: (self.target_image_access, target_layout)..(next_access, next_layout),
                target: image,
                range: image_range,
            })
        } else if !(self.target_image_access.contains(next_access)) {
            self.after_stages |= next_stage;
            self.after_image_access |= next_access;
        }
    }

    pub fn add_buffer(
        &mut self,
        last: Option<(pso::PipelineStage, gfx_hal::buffer::Access)>,
        next: (pso::PipelineStage, gfx_hal::buffer::Access),
    ) {
        if let Some(last) = last {
            if !last.1.contains(self.target_buffer_access) {
                self.before_stages |= last.0;
                self.before_buffer_access |= last.1;
            }
        }
        if !(self.target_buffer_access.contains(next.1)) {
            self.after_stages |= next.0;
            self.after_buffer_access |= next.1;
        }
    }

    pub fn encode_before<C, L>(&mut self, encoder: &mut Encoder<'_, B, C, L>) {
        if self.before_stages != pso::PipelineStage::empty() {
            let transitions = self.before_image_transitions.iter().map(|b| b.raw());
            let all_images = once(gfx_hal::memory::Barrier::AllImages(
                self.before_image_access..self.target_image_access,
            ))
            .filter(|_| self.before_image_access != image::Access::empty());
            let all_buffers = once(gfx_hal::memory::Barrier::AllBuffers(
                self.before_buffer_access..self.target_buffer_access,
            ))
            .filter(|_| self.before_buffer_access != buffer::Access::empty());

            encoder.pipeline_barrier(
                self.before_stages..self.target_stages,
                gfx_hal::memory::Dependencies::empty(),
                transitions.chain(all_images).chain(all_buffers),
            );
        } else {
            assert_eq!(self.before_image_transitions.len(), 0);
        }

        self.before_stages = pso::PipelineStage::empty();
        self.before_image_access = image::Access::empty();
        self.before_buffer_access = buffer::Access::empty();
        self.before_image_transitions.clear();
    }

    pub fn encode_after<C, L>(&mut self, encoder: &mut Encoder<'_, B, C, L>) {
        if self.target_stages != pso::PipelineStage::empty() {
            let transitions = self.after_image_transitions.iter().map(|b| b.raw());
            let all_images = once(gfx_hal::memory::Barrier::AllImages(
                self.target_image_access..self.after_image_access,
            ))
            .filter(|_| self.after_image_access != image::Access::empty());
            let all_buffers = once(gfx_hal::memory::Barrier::AllBuffers(
                self.target_buffer_access..self.after_buffer_access,
            ))
            .filter(|_| self.after_buffer_access != buffer::Access::empty());

            encoder.pipeline_barrier(
                self.target_stages..self.after_stages,
                gfx_hal::memory::Dependencies::empty(),
                transitions.chain(all_images).chain(all_buffers),
            );
        } else {
            assert_eq!(self.after_image_transitions.len(), 0);
        }

        self.after_stages = pso::PipelineStage::empty();
        self.after_image_access = image::Access::empty();
        self.after_buffer_access = buffer::Access::empty();
        self.after_image_transitions.clear();
    }
}
