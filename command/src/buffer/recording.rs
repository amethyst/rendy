
use {
    crate::{
        encoder::{Encoder, EncoderCommon, RenderPassEncoder, RenderPassEncoderHRTB},
        capability::{Supports, Graphics, Transfer, Compute},
    },
    super::{
        CommandBuffer, Submittable,
        state::{ExecutableState, RecordingState},
        usage::Usage,
    },
};

impl<B, C, U, P, L, R> CommandBuffer<B, C, RecordingState<U, P>, L, R>
where
    B: gfx_hal::Backend,
{
    /// Finish recording command buffer.
    ///
    /// # Parameters
    pub fn finish(
        mut self,
    ) -> CommandBuffer<B, C, ExecutableState<U, P>, L, R>
    where
        U: Usage,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::finish(self.raw());
            self.change_state(|RecordingState(usage, pass_continue)| ExecutableState(usage, pass_continue))
        }
    }
}

impl<B, C, U, P, L, R> EncoderCommon<B, C> for CommandBuffer<B, C, RecordingState<U, P>, L, R>
where
    B: gfx_hal::Backend,
{
    fn bind_index_buffer(&mut self, buffer: &B::Buffer, offset: u64, index_type: gfx_hal::IndexType)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_index_buffer(
                self.raw(),
                gfx_hal::buffer::IndexBufferView {
                    buffer: buffer,
                    offset,
                    index_type,
                }
            )
        }
    }

    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b B::Buffer, u64)>)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_vertex_buffers(
                self.raw(),
                first_binding,
                buffers,
            )
        }
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_graphics_pipeline(&mut self.raw, pipeline);
        }
    }

    fn bind_graphics_descriptor_sets<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'b B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    )
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_graphics_descriptor_sets(
                &mut self.raw,
                layout,
                first_set as _,
                sets,
                offsets,
            );
        }
    }

    fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline)
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_compute_pipeline(&mut self.raw, pipeline);
        }
    }

    fn bind_compute_descriptor_sets<'a>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'a B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    )
    where
        C: Supports<Compute>,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_compute_descriptor_sets(
                &mut self.raw,
                layout,
                first_set as usize,
                sets,
                offsets,
            );
        }
    }

	
	fn pipeline_barrier<'a>(
		&mut self,
        stages: std::ops::Range<gfx_hal::pso::PipelineStage>,
        dependencies: gfx_hal::memory::Dependencies,
        barriers: impl IntoIterator<Item = gfx_hal::memory::Barrier<'a, B>>,
	) {
		unsafe {
			gfx_hal::command::RawCommandBuffer::pipeline_barrier(
				&mut self.raw,
				stages,
				dependencies,
				barriers,
			)
		}
	}

	fn execute_commands(&mut self, submittables: impl IntoIterator<Item = impl Submittable<B>>) {
        let submittables: smallvec::SmallVec<[_; 16]> = submittables.into_iter().inspect(|submittable| {
            assert_eq!(self.family, submittable.family());
        }).collect();
        unsafe {
			gfx_hal::command::RawCommandBuffer::execute_commands(
				&mut self.raw,
                submittables.iter().map(Submittable::raw)
			)
		}
    }
}

impl<'a, B, C, U, L, R> RenderPassEncoderHRTB<'a, B, C> for CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: gfx_hal::Backend,
{
    type RenderPassEncoder = RenderPassInlineEncoder<'a, B>;
}

impl<B, C, U, L, R> Encoder<B, C> for CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: gfx_hal::Backend,
{
    fn begin_render_pass_inline<'a>(
        &'a mut self,
        render_pass: &B::RenderPass, 
        framebuffer: &B::Framebuffer, 
        render_area: gfx_hal::pso::Rect, 
        clear_values: &[gfx_hal::command::ClearValueRaw],
    ) -> RenderPassInlineEncoder<'a, B>
    where
        C: Supports<Graphics>,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::begin_render_pass(
                self.raw(),
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                gfx_hal::command::SubpassContents::Inline,
            )
        }

        RenderPassInlineEncoder {
            family: self.family,
            raw: unsafe { self.raw() },
        }
    }

    fn copy_image(
        &mut self, 
        src: &B::Image, 
        src_layout: gfx_hal::image::Layout, 
        dst: &B::Image, 
        dst_layout: gfx_hal::image::Layout, 
        regions: impl IntoIterator<Item = gfx_hal::command::ImageCopy>
    )
    where
        C: Supports<Transfer>,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::copy_image(
                self.raw(),
                src,
                src_layout,
                dst,
                dst_layout,
                regions,
            )
        }
    }

    fn dispatch(&mut self, x: u32, y: u32, z: u32)
    where
        C: Supports<Compute>,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::dispatch(
                self.raw(),
                [x, y, z],
            )
        }
    }

    fn dispatch_indirect(&mut self, buffer: &B::Buffer, offset: u64)
    where
        C: Supports<Compute>,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::dispatch_indirect(
                self.raw(),
                buffer,
                offset,
            )
        }
    }
}

#[derive(Debug)]
pub struct RenderPassInlineEncoder<'a, B: gfx_hal::Backend> {
    raw: &'a mut B::CommandBuffer,
    family: gfx_hal::queue::QueueFamilyId,
}

impl<'a, B> EncoderCommon<B, Graphics> for RenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn bind_index_buffer(&mut self, buffer: &B::Buffer, offset: u64, index_type: gfx_hal::IndexType) {
        gfx_hal::command::RawCommandBuffer::bind_index_buffer(
            self.raw,
            gfx_hal::buffer::IndexBufferView {
                buffer: buffer,
                offset,
                index_type,
            }
        )
    }

    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b B::Buffer, u64)>) {
        gfx_hal::command::RawCommandBuffer::bind_vertex_buffers(
            self.raw,
            first_binding,
            buffers,
        )
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_graphics_pipeline(self.raw, pipeline);
        }
    }

    fn bind_graphics_descriptor_sets<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'b B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    ) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_graphics_descriptor_sets(
                self.raw,
                layout,
                first_set as _,
                sets,
                offsets,
            );
        }
    }

    fn bind_compute_pipeline(&mut self, _pipeline: &B::ComputePipeline) {
        unsafe { // No way to call this function.
            std::hint::unreachable_unchecked()
        }
    }

    fn bind_compute_descriptor_sets<'b>(
        &mut self,
        _layout: &B::PipelineLayout,
        _first_set: u32,
        _sets: impl IntoIterator<Item = &'b B::DescriptorSet>,
        _offsets: impl IntoIterator<Item = u32>,
    ) {
        unsafe { // No way to call this function.
            std::hint::unreachable_unchecked()
        }
    }

	
	fn pipeline_barrier<'b>(
		&mut self,
        stages: std::ops::Range<gfx_hal::pso::PipelineStage>,
        dependencies: gfx_hal::memory::Dependencies,
        barriers: impl IntoIterator<Item = gfx_hal::memory::Barrier<'b, B>>,
	) {
		unsafe {
			gfx_hal::command::RawCommandBuffer::pipeline_barrier(
				self.raw,
				stages,
				dependencies,
				barriers,
			)
		}
	}

	fn execute_commands(&mut self, submittables: impl IntoIterator<Item = impl Submittable<B>>) {
        let submittables: smallvec::SmallVec<[_; 16]> = submittables.into_iter().inspect(|submittable| {
            assert_eq!(self.family, submittable.family());
        }).collect();
        unsafe {
			gfx_hal::command::RawCommandBuffer::execute_commands(
				self.raw,
                submittables.iter().map(Submittable::raw)
			)
		}
    }
}

impl<'a, B> RenderPassEncoder<B> for RenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn draw(
        &mut self, 
        vertices: std::ops::Range<u32>, 
        instances: std::ops::Range<u32>,
    ) {
        gfx_hal::command::RawCommandBuffer::draw(
            self.raw,
            vertices,
            instances,
        )
    }

    fn draw_indexed(
        &mut self, 
        indices: std::ops::Range<u32>, 
        base_vertex: i32, 
        instances: std::ops::Range<u32>,
    ) {
        gfx_hal::command::RawCommandBuffer::draw_indexed(
            self.raw,
            indices,
            base_vertex,
            instances,
        )
    }

    fn draw_indirect(
        &mut self,
        buffer: &B::Buffer, 
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        gfx_hal::command::RawCommandBuffer::draw_indirect(
            self.raw,
            buffer,
            offset,
            draw_count,
            stride,
        )
    }
}
