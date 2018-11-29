
use {
    crate::{
        encoder::{Encoder, EncoderCommon, RenderPassEncoder, RenderPassEncoderHRTB},
        capability::{Supports, Graphics, Transfer},
        resource::{Buffer, Image},
    },
    super::{
        CommandBuffer,
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
    fn bind_index_buffer(&mut self, buffer: &Buffer<B>, offset: u64, index_type: gfx_hal::IndexType)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_index_buffer(
                self.raw(),
                gfx_hal::buffer::IndexBufferView {
                    buffer: buffer.raw(),
                    offset,
                    index_type,
                }
            )
        }
    }

    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b Buffer<B>, u64)>)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_vertex_buffers(
                self.raw(),
                first_binding,
                buffers.into_iter().map(|(buffer, offset)| (buffer.raw(), offset)),
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
}

#[derive(Debug)]
pub struct RenderPassInlineEncoder<'a, B: gfx_hal::Backend> {
    raw: &'a mut B::CommandBuffer,
}

impl<'a, B> EncoderCommon<B, Graphics> for RenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn bind_index_buffer(&mut self, buffer: &Buffer<B>, offset: u64, index_type: gfx_hal::IndexType) {
        gfx_hal::command::RawCommandBuffer::bind_index_buffer(
            self.raw,
            gfx_hal::buffer::IndexBufferView {
                buffer: buffer.raw(),
                offset,
                index_type,
            }
        )
    }

    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b Buffer<B>, u64)>) {
        gfx_hal::command::RawCommandBuffer::bind_vertex_buffers(
            self.raw,
            first_binding,
            buffers.into_iter().map(|(buffer, offset)| (buffer.raw(), offset)),
        )
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_graphics_pipeline(self.raw, pipeline);
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
}