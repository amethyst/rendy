use std::collections::VecDeque;

use crate::{
    command::{
        Capability, CommandBuffer, CommandPool, ExecutableState, IndividualReset, RecordingState, Submit,
        InitialState, Level, MultiShot, PendingState, Usage, PrimaryLevel,
        EncoderCommon, RenderPassEncoder, Encoder, Supports, Graphics, RenderPassEncoderHRTB, Transfer,
        Compute,
    },
};

/// Command ring buffer.
#[derive(Debug)]
pub struct CommandCirque<B: gfx_hal::Backend, C, S = (), P = (), L = PrimaryLevel> {
    pendings: VecDeque<Pending<B, C, S, P, L>>,
    executables: VecDeque<Executable<B, C, S, P, L>>,
    level: L,
    counter: usize,
}

#[derive(Debug)]
struct Pending<B: gfx_hal::Backend, C, S, P, L> {
    buffer: CommandBuffer<B, C, PendingState<ExecutableState<MultiShot<S>, P>>, L, IndividualReset>,
    index: usize,
    frame: u64,
}

#[derive(Debug)]
struct Executable<B: gfx_hal::Backend, C, S, P, L> {
    buffer: CommandBuffer<B, C, ExecutableState<MultiShot<S>, P>, L, IndividualReset>,
    index: usize,
}

impl<B, C, S, P, L> CommandCirque<B, C, S, P, L>
where
    B: gfx_hal::Backend,
    C: Capability,
    L: Level,
{
    /// Create new command cirque for pool.
    pub fn new(level: L) -> Self {
        CommandCirque {
            pendings: VecDeque::new(),
            executables: VecDeque::new(),
            level,
            counter: 0,
        }
    }

    /// All buffers must complete.
    /// Usually this function is called after waiting for device idle.
    pub unsafe fn dispose(mut self, pool: &mut CommandPool<B, C, IndividualReset>) {
        pool.free_buffers(self.pendings.drain(..).map(|p| p.buffer.complete()).chain(self.executables.drain(..).map(|e| e.buffer)));
    }

    /// Get executable buffer from this cirque.
    /// 
    /// # Parameters
    /// 
    /// `frames` - range of frame indices. `oldest_pending_frame .. next_frame`.
    /// Typically obtained with `Frames::range()`.
    ///
    /// # Safety
    /// 
    /// ???
    pub unsafe fn get(
        &mut self,
        frames: std::ops::Range<u64>,
        pool: &mut CommandPool<B, C, IndividualReset>,
    ) -> either::Either<
        CirqueEncoder<'_, B, C, ExecutableState<MultiShot<S>, P>, S, P, L>,
        CirqueEncoder<'_, B, C, InitialState, S, P, L>,
    > {
        while self
            .pendings
            .front()
            .as_ref()
            .map_or(false, |pending| pending.frame < frames.start)
        {
            let pending = self.pendings.pop_front().unwrap();
            // All commands from this buffer are complete.
            let buffer = pending.buffer.complete();
            self.executables.push_back(Executable {
                buffer,
                index: pending.index,
            });
        }

        if let Some(executable) = self.executables.pop_front() {
            either::Left(CirqueEncoder {
                buffer: executable.buffer,
                index: executable.index,
                cirque: self,
                frame: frames.end,
            })
        } else {
            let buffer = pool.allocate_buffers(self.level, 1).remove(0);
            self.counter += 1;

            either::Right(CirqueEncoder {
                buffer,
                index: self.counter,
                cirque: self,
                frame: frames.end,
            })
        }
    }
}

/// Buffer borrowed from `CommandCirque`.
#[derive(Debug)]
pub struct CirqueEncoder<'a, B: gfx_hal::Backend, C, X = RecordingState<MultiShot>, S = (), P = (), L = PrimaryLevel> {
    buffer: CommandBuffer<B, C, X, L, IndividualReset>,
    frame: u64,
    cirque: &'a mut CommandCirque<B, C, S, P, L>,
    index: usize,
}

impl<'a, B, C, X, S, P, L> CirqueEncoder<'a, B, C, X, S, P, L>
where
    B: gfx_hal::Backend,
{
    /// Cirque index of the encoder.
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<'a, B, C, S, P, L> CirqueEncoder<'a, B, C, InitialState, S, P, L>
where
    B: gfx_hal::Backend,
{
    /// Begin recording command buffer.
    pub fn begin(self) -> CirqueEncoder<'a, B, C, RecordingState<MultiShot<S>, P>, S, P, L>
    where
        MultiShot<S>: Usage,
        P: Usage,
    {
        CirqueEncoder {
            buffer: self.buffer.begin(Default::default(), Default::default()),
            frame: self.frame,
            cirque: self.cirque,
            index: self.index,
        }
    }
}

impl<'a, B, C, S, P, L> EncoderCommon<B, C> for CirqueEncoder<'a, B, C, RecordingState<MultiShot<S>, P>, S, P, L>
where
    B: gfx_hal::Backend,
{
    fn bind_index_buffer(&mut self, buffer: &B::Buffer, offset: u64, index_type: gfx_hal::IndexType)
    where
        C: Supports<Graphics>,
    {
        self.buffer.bind_index_buffer(buffer, offset, index_type)
    }

    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b B::Buffer, u64)>)
    where
        C: Supports<Graphics>,
    {
        self.buffer.bind_vertex_buffers(first_binding, buffers)
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline)
    where
        C: Supports<Graphics>,
    {
        self.buffer.bind_graphics_pipeline(pipeline)
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
        self.buffer.bind_graphics_descriptor_sets(layout, first_set, sets, offsets)
    }

    fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline)
    where
        C: Supports<Compute>,
    {
        self.buffer.bind_compute_pipeline(pipeline)
    }

    fn bind_compute_descriptor_sets<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'b B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    )
    where
        C: Supports<Compute>,
    {
        self.buffer.bind_compute_descriptor_sets(layout, first_set, sets, offsets)
    }
}

impl<'a, 'b, B, C, S, P, L>  RenderPassEncoderHRTB<'b, B, C> for CirqueEncoder<'a, B, C, RecordingState<MultiShot<S>, P>, S, P, L>
where
    B: gfx_hal::Backend,
{
    type RenderPassEncoder = CirqueRenderPassInlineEncoder<'b, B>;
}

impl<'a, B, C, S, P, L> Encoder<B, C> for CirqueEncoder<'a, B, C, RecordingState<MultiShot<S>, P>, S, P, L>
where
    B: gfx_hal::Backend,
{
    fn begin_render_pass_inline(
        &mut self,
        render_pass: &B::RenderPass, 
        framebuffer: &B::Framebuffer, 
        render_area: gfx_hal::pso::Rect, 
        clear_values: &[gfx_hal::command::ClearValueRaw],
    ) -> CirqueRenderPassInlineEncoder<'_, B>
    where
        C: Supports<Graphics>,
    {
        self.buffer.capability().assert();

        let buffer = unsafe { self.buffer.raw() };
        unsafe {
            gfx_hal::command::RawCommandBuffer::begin_render_pass(
                buffer,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                gfx_hal::command::SubpassContents::Inline,
            );

            // `CirqueRenderPassInlineEncoder` allows to record only render pass commands.
            CirqueRenderPassInlineEncoder {
                buffer,
                index: self.index,
            }
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
                self.buffer.raw(),
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
                self.buffer.raw(),
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
                self.buffer.raw(),
                buffer,
                offset,
            )
        }
    }
}

impl<'a, B, C, S, P, L> CirqueEncoder<'a, B, C, RecordingState<MultiShot<S>, P>, S, P, L>
where
    B: gfx_hal::Backend,
{
    /// Finish recording command buffer.
    pub fn finish(self) -> CirqueEncoder<'a, B, C, ExecutableState<MultiShot<S>, P>, S, P, L>
    where
        MultiShot<S>: Usage,
        P: Usage,
    {
        CirqueEncoder {
            buffer: self.buffer.finish(),
            frame: self.frame,
            cirque: self.cirque,
            index: self.index,
        }
    }
}

impl<'a, B, C, S, P, L> CirqueEncoder<'a, B, C, ExecutableState<MultiShot<S>, P>, S, P, L>
where
    B: gfx_hal::Backend,
    S: Copy,
    P: Copy,
    L: Copy,
{
    /// Reset command buffer.
    pub fn reset(self) -> CirqueEncoder<'a, B, C, InitialState, S, P, L> {
        CirqueEncoder {
            buffer: self.buffer.reset(),
            frame: self.frame,
            cirque: self.cirque,
            index: self.index,
        }
    }

    /// Submit commands.
    /// This function creates submit instance bound to `Frames` reference lifetime.
    /// This guarantees that it will be submitted during current frame.
    /// Or not submitted at all.
    ///
    /// Command buffer is returned to `CommandCirque` as pending with frame index attached.
    /// Once frame is complete command buffer can be reused.
    pub fn submit(self) -> Submit<'static, B, S, P, L> {
        let (submit, buffer) = self.buffer.submit();
        self.cirque.pendings.push_back(Pending {
            buffer,
            index: self.index,
            frame: self.frame,
        });

        submit
    }
}

/// Borrowed buffer from `CommandCirque` ready for render pass encoding.
#[derive(Debug)]
pub struct CirqueRenderPassInlineEncoder<'a, B: gfx_hal::Backend> {
    buffer: &'a mut B::CommandBuffer,
    index: usize,
}

impl<'a, B> CirqueRenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    /// Get cirque index of the encoder.
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<'a, B> EncoderCommon<B, Graphics> for CirqueRenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn bind_index_buffer(&mut self, buffer: &B::Buffer, offset: u64, index_type: gfx_hal::IndexType) {
        gfx_hal::command::RawCommandBuffer::bind_index_buffer(
            self.buffer,
            gfx_hal::buffer::IndexBufferView {
                buffer,
                offset,
                index_type,
            }
        )
    }

    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b B::Buffer, u64)>) {
        gfx_hal::command::RawCommandBuffer::bind_vertex_buffers(
            self.buffer,
            first_binding,
            buffers,
        )
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_graphics_pipeline(self.buffer, pipeline);
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
                self.buffer,
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
}

impl<'a, B> RenderPassEncoder<B> for CirqueRenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn draw(
        &mut self, 
        vertices: std::ops::Range<u32>, 
        instances: std::ops::Range<u32>,
    ) {
        gfx_hal::command::RawCommandBuffer::draw(
            self.buffer,
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
            self.buffer,
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
            self.buffer,
            buffer,
            offset,
            draw_count,
            stride,
        )
    }
}

impl<'a, B> Drop for CirqueRenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn drop(&mut self) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::end_render_pass(
                self.buffer
            )
        }
    }
}

