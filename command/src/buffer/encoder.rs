
use {
    super::{
        submit::Submittable,
        level::{Level, PrimaryLevel, SecondaryLevel},
        state::RecordingState,
        usage::RenderPassContinue,
        CommandBuffer,
    },
    crate::{
        capability::{Supports, Graphics, Transfer, Compute, Capability},
        family::FamilyId,
    },
};

/// Draw command for indirect draw.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrawCommand {
    /// Number of vertices to draw.
    pub vertex_count: u32,

    /// Number of instanced to draw.
    pub instance_count: u32,

    /// First vertex index.
    pub first_vertex: u32,

    /// First instance index.
    pub first_instance: u32,
}

/// Draw command for indirect indexed draw.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrawIndexedCommand {
    /// Number of indices to draw.
    pub index_count: u32,

    /// Number of instances to draw.
    pub instance_count: u32,

    /// First index.
    pub first_index: u32,

    /// Vertex offset that is added to index before indexing the vertex buffer.
    pub vertex_offset: i32,

    /// First instance index.
    pub first_instance: u32,
}

/// Draw command for dispatch.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DispatchCommand {
    /// Number of local workgroups to dispatch in the X dimension.
    pub x: u32,

    /// Number of local workgroups to dispatch in the Y dimension.
    pub y: u32,

    /// Number of local workgroups to dispatch in the Z dimension.
    pub z: u32,
}

/// Encoder for recording commands inside or outside renderpass.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct EncoderCommon<'a, B: gfx_hal::Backend, C> {
    #[derivative(Debug = "ignore")] raw: &'a mut B::CommandBuffer,
    capability: C,
    family: FamilyId,
}

impl<'a, B, C> EncoderCommon<'a, B, C>
where
    B: gfx_hal::Backend,
{
    /// Bind index buffer.
    pub fn bind_index_buffer<'b>(&mut self, buffer: &'b B::Buffer, offset: u64, index_type: gfx_hal::IndexType)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_index_buffer(
                self.raw,
                gfx_hal::buffer::IndexBufferView {
                    buffer: buffer,
                    offset,
                    index_type,
                }
            )
        }
    }

    /// Bind vertex buffers.
    pub fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b B::Buffer, u64)>)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_vertex_buffers(
                self.raw,
                first_binding,
                buffers,
            )
        }
    }

    /// Bind graphics pipeline.
    pub fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_graphics_pipeline(self.raw, pipeline);
        }
    }

    /// Bind descriptor sets to graphics pipeline.
    pub fn bind_graphics_descriptor_sets<'b>(
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
                self.raw,
                layout,
                first_set as _,
                sets,
                offsets,
            );
        }
    }

    /// Bind graphics pipeline.
    pub fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline)
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_compute_pipeline(self.raw, pipeline);
        }
    }

    /// Bind descriptor sets to compute pipeline.
    pub fn bind_compute_descriptor_sets<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'b B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    )
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::bind_compute_descriptor_sets(
                self.raw,
                layout,
                first_set as usize,
                sets,
                offsets,
            );
        }
    }

	/// Insert pipeline barrier.
	pub fn pipeline_barrier<'b>(
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
    
    /// Push graphics constants.
    pub fn push_constants<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        stages: gfx_hal::pso::ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    ) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::push_graphics_constants(
                self.raw,
                layout,
                stages,
                offset,
                constants
            );
        }
    }

    /// Reborrow encoder.
    pub fn reborrow<K>(&mut self) -> EncoderCommon<'_, B, K>
    where
        C: Supports<K>,
    {
        EncoderCommon {
            capability: self.capability.supports().unwrap(),
            raw: &mut*self.raw,
            family: self.family,
        }
    }
}

/// Special encoder to record render-pass commands.
#[derive(Debug)]
pub struct RenderPassEncoder<'a, B: gfx_hal::Backend> {
    inner: EncoderCommon<'a, B, Graphics>,
}

impl<'a, B> std::ops::Deref for RenderPassEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    type Target = EncoderCommon<'a, B, Graphics>;

    fn deref(&self) -> &EncoderCommon<'a, B, Graphics> {
        &self.inner
    }
}

impl<'a, B> std::ops::DerefMut for RenderPassEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn deref_mut(&mut self) -> &mut EncoderCommon<'a, B, Graphics> {
        &mut self.inner
    }
}

impl<'a, B> RenderPassEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    /// Draw.
    pub fn draw(
        &mut self, 
        vertices: std::ops::Range<u32>, 
        instances: std::ops::Range<u32>,
    ) {
        unsafe { gfx_hal::command::RawCommandBuffer::draw(
            self.inner.raw,
            vertices,
            instances,
        ) }
    }

    /// Draw indexed.
    pub fn draw_indexed(
        &mut self, 
        indices: std::ops::Range<u32>, 
        base_vertex: i32, 
        instances: std::ops::Range<u32>,
    ) {
        unsafe { gfx_hal::command::RawCommandBuffer::draw_indexed(
            self.inner.raw,
            indices,
            base_vertex,
            instances,
        ) }
    }

    /// Draw indirect.
    /// Similar to [`draw`] except takes vertices and indices from `buffer` at specified `offset`.
    /// `buffer` must contain `draw_count` of [`DrawCommand`] starting from `offset` with `stride` bytes between each.
    /// 
    /// [`draw`]: trait.RenderPassInlineEncoder.html#tymethod.draw
    /// [`DrawCommand`]: struct.DrawCommand.html
    pub fn draw_indirect(
        &mut self, 
        buffer: &B::Buffer, 
        offset: u64, 
        draw_count: u32, 
        stride: u32,
    ) {
        unsafe { gfx_hal::command::RawCommandBuffer::draw_indirect(
            self.inner.raw,
            buffer,
            offset,
            draw_count,
            stride,
        ) }
    }

    /// Draw indirect.
    /// Similar to [`draw`] except takes vertices and indices from `buffer` at specified `offset`.
    /// `buffer` must contain `draw_count` of [`DrawCommand`] starting from `offset` with `stride` bytes between each.
    /// 
    /// [`draw`]: trait.RenderPassInlineEncoder.html#tymethod.draw
    /// [`DrawCommand`]: struct.DrawCommand.html
    pub fn draw_indexed_indirect(
        &mut self, 
        buffer: &B::Buffer, 
        offset: u64, 
        draw_count: u32, 
        stride: u32,
    ) {
        unsafe { gfx_hal::command::RawCommandBuffer::draw_indexed_indirect(
            self.inner.raw,
            buffer,
            offset,
            draw_count,
            stride,
        ) }
    }

    /// Reborrow encoder.
    pub fn reborrow(&mut self) -> RenderPassEncoder<'_, B> {
        RenderPassEncoder {
            inner: self.inner.reborrow()
        }
    }
}

/// Special encoder to record commands inside render pass.
#[derive(Debug)]
pub struct RenderPassInlineEncoder<'a, B: gfx_hal::Backend> {
    inner: RenderPassEncoder<'a, B>,
}

impl<'a, B> Drop for RenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn drop(&mut self) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::end_render_pass(
                self.inner.inner.raw,
            )
        }
    }
}

impl<'a, B> std::ops::Deref for RenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    type Target = RenderPassEncoder<'a, B>;

    fn deref(&self) -> &RenderPassEncoder<'a, B> {
        &self.inner
    }
}

impl<'a, B> std::ops::DerefMut for RenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn deref_mut(&mut self) -> &mut RenderPassEncoder<'a, B> {
        &mut self.inner
    }
}

impl<'a, B> RenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    /// Record next subpass inline.
    pub fn next_subpass_inline(self) -> RenderPassInlineEncoder<'a, B> {
        unsafe {
            gfx_hal::command::RawCommandBuffer::next_subpass(
                self.inner.inner.raw,
                gfx_hal::command::SubpassContents::Inline,
            );
        }

        self
    }

    /// Record next subpass secondary.
    pub fn next_subpass_secondary(self) -> RenderPassSecondaryEncoder<'a, B> {
        unsafe {
            gfx_hal::command::RawCommandBuffer::next_subpass(
                self.inner.inner.raw,
                gfx_hal::command::SubpassContents::SecondaryBuffers,
            );
        }

        unsafe {
            let next = RenderPassSecondaryEncoder {
                inner: std::ptr::read(&self.inner.inner),
            };

            std::mem::forget(self);
            next
        }
    }
}

/// Special encoder to execute secondary buffers inside render pass.
#[derive(Debug)]
pub struct RenderPassSecondaryEncoder<'a, B: gfx_hal::Backend> {
    inner: EncoderCommon<'a, B, Graphics>,
}

impl<'a, B> Drop for RenderPassSecondaryEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn drop(&mut self) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::end_render_pass(
                self.inner.raw,
            )
        }
    }
}

impl<'a, B> RenderPassSecondaryEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    /// Execute commands from secondary buffers.
	pub fn execute_commands(
        &mut self,
        submittables: impl IntoIterator<Item = impl Submittable<B, SecondaryLevel, RenderPassContinue>>
    ) {
        let family = self.inner.family;
        unsafe {
			gfx_hal::command::RawCommandBuffer::execute_commands(
				self.inner.raw,
                submittables.into_iter().map(|submit| {
                    assert_eq!(family, submit.family());
                    submit.raw()
                })
			)
		}
    }

    /// Record next subpass inline.
    pub fn next_subpass_inline(self) -> RenderPassInlineEncoder<'a, B> {
        unsafe {
            gfx_hal::command::RawCommandBuffer::next_subpass(
                self.inner.raw,
                gfx_hal::command::SubpassContents::Inline,
            );
        }

        unsafe {
            let next = RenderPassInlineEncoder {
                inner: RenderPassEncoder {
                    inner: std::ptr::read(&self.inner),
                }
            };

            std::mem::forget(self);
            next
        }
    }

    /// Record next subpass secondary.
    pub fn next_subpass_secondary(self) -> RenderPassSecondaryEncoder<'a, B> {
        unsafe {
            gfx_hal::command::RawCommandBuffer::next_subpass(
                self.inner.raw,
                gfx_hal::command::SubpassContents::SecondaryBuffers,
            );
        }

        self
    }
}

/// Trait to encode commands outside render pass.
#[derive(Debug)]
pub struct Encoder<'a, B: gfx_hal::Backend, C, L> {
    inner: EncoderCommon<'a, B, C>,
    level: L,
}

impl<'a, B, C, L> std::ops::Deref for Encoder<'a, B, C, L>
where
    B: gfx_hal::Backend,
{
    type Target = EncoderCommon<'a, B, C>;

    fn deref(&self) -> &EncoderCommon<'a, B, C> {
        &self.inner
    }
}

impl<'a, B, C, L> std::ops::DerefMut for Encoder<'a, B, C, L>
where
    B: gfx_hal::Backend,
{
    fn deref_mut(&mut self) -> &mut EncoderCommon<'a, B, C> {
        &mut self.inner
    }
}

impl<'a, B, C> Encoder<'a, B, C, PrimaryLevel>
where
    B: gfx_hal::Backend,
{
    /// Beging recording render pass inline.
    pub fn begin_render_pass_inline(
        &mut self,
        render_pass: &B::RenderPass, 
        framebuffer: &B::Framebuffer, 
        render_area: gfx_hal::pso::Rect, 
        clear_values: &[gfx_hal::command::ClearValueRaw],
    ) -> RenderPassInlineEncoder<'_, B>
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::begin_render_pass(
                self.inner.raw,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                gfx_hal::command::SubpassContents::Inline,
            )
        }

        RenderPassInlineEncoder {
            inner: RenderPassEncoder {
                inner: self.inner.reborrow(),
            }
        }
    }

    /// Beging recording render pass secondary.
    pub fn begin_render_pass_secondary(
        &mut self,
        render_pass: &B::RenderPass, 
        framebuffer: &B::Framebuffer, 
        render_area: gfx_hal::pso::Rect, 
        clear_values: &[gfx_hal::command::ClearValueRaw],
    ) -> RenderPassSecondaryEncoder<'_, B>
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::begin_render_pass(
                self.inner.raw,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                gfx_hal::command::SubpassContents::SecondaryBuffers,
            )
        }

        RenderPassSecondaryEncoder {
            inner: self.inner.reborrow(),
        }
    }

    /// Execute commands from secondary buffers.
	pub fn execute_commands(
        &mut self,
        submittables: impl IntoIterator<Item = impl Submittable<B, SecondaryLevel>>
    ) {
        let family = self.inner.family;
        unsafe {
			gfx_hal::command::RawCommandBuffer::execute_commands(
				self.inner.raw,
                submittables.into_iter().map(|submit| {
                    assert_eq!(family, submit.family());
                    submit.raw()
                })
			)
		}
    }
}

impl<'a, B, C, L> Encoder<'a, B, C, L>
where
    B: gfx_hal::Backend,
{
    /// Get encoder level.
    pub fn level(&self) -> L
    where
        L: Level,
    {
        self.level
    }

    /// Copy buffer regions.
    /// `src` and `dst` can be the same buffer or alias in memory.
    /// But regions must not overlap.
    /// Otherwise resulting values are undefined.
    pub fn copy_buffer(
        &mut self,
        src: &B::Buffer,
        dst: &B::Buffer,
        regions: impl IntoIterator<Item = gfx_hal::command::BufferCopy>,
    )
    where
        C: Supports<Transfer>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::copy_buffer(
                self.inner.raw,
                src,
                dst,
                regions,
            )
        }
    }

    /// Copy buffer region to image subresource range.
    pub fn copy_buffer_to_image(
        &mut self, 
        src: &B::Buffer, 
        dst: &B::Image, 
        dst_layout: gfx_hal::image::Layout,
        regions: impl IntoIterator<Item = gfx_hal::command::BufferImageCopy>
    )
    where
        C: Supports<Transfer>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::copy_buffer_to_image(
                self.inner.raw,
                src,
                dst,
                dst_layout,
                regions,
            )
        }
    }

    /// Copy image regions.
    pub fn copy_image(
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
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::copy_image(
                self.inner.raw,
                src,
                src_layout,
                dst,
                dst_layout,
                regions,
            )
        }
    }

    /// Dispatch compute.
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32)
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::dispatch(
                self.inner.raw,
                [x, y, z],
            )
        }
    }

    /// Dispatch indirect.
    /// Similar to [`dispatch`] except takes vertices and indices from `buffer` at specified `offset`.
    /// `buffer` must contain [`DispatchCommand`] at `offset`.
    /// 
    /// [`dispatch`]: trait.Encoder.html#tymethod.dispatch
    /// [`DispatchCommand`]: struct.DispatchCommand.html
    pub fn dispatch_indirect(&mut self, buffer: &B::Buffer, offset: u64)
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        unsafe {
            gfx_hal::command::RawCommandBuffer::dispatch_indirect(
                self.inner.raw,
                buffer,
                offset,
            )
        }
    }
}

impl<B, C, U, L, R> CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: gfx_hal::Backend,
    C: Capability,
    L: Level,
{
    /// Get encoder that will encode commands into this command buffer.
    pub fn encoder(&mut self) -> Encoder<'_, B, C, L> {
        Encoder {
            level: self.level,
            inner: EncoderCommon {
                capability: self.capability,
                family: self.family,
                raw: self.raw(),
            },
        }
    }
}

impl<B, C, U, R> CommandBuffer<B, C, RecordingState<U, RenderPassContinue>, SecondaryLevel, R>
where
    B: gfx_hal::Backend,
    C: Supports<Graphics>,
{
    /// Get encoder that will encode render-pass commands into this command buffer.
    pub fn render_pass_encoder(&mut self) -> RenderPassEncoder<'_, B> {
        RenderPassEncoder {
            inner: EncoderCommon {
                capability: self.capability.supports().unwrap(),
                family: self.family,
                raw: self.raw(),
            },
        }
    }
}
