use {
    super::{
        level::{Level, PrimaryLevel, SecondaryLevel},
        state::RecordingState,
        submit::Submittable,
        usage::RenderPassContinue,
        CommandBuffer,
    },
    crate::{
        capability::{Capability, Compute, Graphics, Supports, Transfer},
        family::FamilyId,
    },
};

/// Draw command for [`draw_indirect`].
///
/// [`draw_indirect`]: ../struct.RenderPassEncoder.html#method.draw_indirect
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

/// Draw command for [`draw_indexed_indirect`].
///
/// [`draw_indexed_indirect`]: ../struct.RenderPassEncoder.html#method.draw_indexed_indirect
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
#[derive(Debug)]
pub struct EncoderCommon<'a, B: rendy_core::hal::Backend, C> {
    raw: &'a mut B::CommandBuffer,
    capability: C,
    family: FamilyId,
}

impl<'a, B, C> EncoderCommon<'a, B, C>
where
    B: rendy_core::hal::Backend,
{
    /// Bind index buffer.
    /// Last bound index buffer is used in [`draw_indexed`] command.
    ///
    /// Note that `draw*` commands available only inside renderpass.
    ///
    /// [`draw_indexed`]: ../struct.RenderPassEncoder.html#method.draw_indexed
    ///
    /// # Safety
    ///
    /// `offset` must not be greater than the size of `buffer`.
    /// Sum of `offset` and starting address of the `buffer` must be
    /// multiple of index size indicated by `index_type`.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdBindIndexBuffer.html
    pub unsafe fn bind_index_buffer<'b>(
        &mut self,
        buffer: &'b B::Buffer,
        offset: u64,
        index_type: rendy_core::hal::IndexType,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        let range = rendy_core::hal::buffer::SubRange { offset, size: None };
        rendy_core::hal::command::CommandBuffer::bind_index_buffer(
            self.raw,
            rendy_core::hal::buffer::IndexBufferView {
                buffer: buffer,
                range,
                index_type,
            },
        )
    }

    /// Bind vertex buffers.
    /// Last bound vertex buffer is used in [`draw`] and [`draw_indexed`] commands.
    ///
    /// Note that `draw*` commands available only inside renderpass.
    ///
    /// [`draw`]: ../struct.RenderPassEncoder.html#method.draw
    /// [`draw_indexed`]: ../struct.RenderPassEncoder.html#method.draw_indexed
    ///
    /// # Safety
    ///
    /// `first_binding + buffers.into_iter().count()` must less than or equal to the `maxVertexInputBindings`
    /// device limit.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdBindVertexBuffers.html
    pub unsafe fn bind_vertex_buffers<'b>(
        &mut self,
        first_binding: u32,
        buffers: impl IntoIterator<Item = (&'b B::Buffer, rendy_core::hal::buffer::SubRange)>,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::bind_vertex_buffers(
            self.raw,
            first_binding,
            buffers,
        )
    }

    /// Bind graphics pipeline.
    ///
    /// Last bound vertex buffer is used in [`draw`], [`draw_indexed`], [`draw_indirect`] and [`draw_indexed_indirect`] commands.
    ///
    /// [`draw_indexed_indirect`]: ../struct.RenderPassEncoder.html#method.draw_indexed_indirect
    /// [`draw_indirect`]: ../struct.RenderPassEncoder.html#method.draw_indirect
    pub fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            rendy_core::hal::command::CommandBuffer::bind_graphics_pipeline(self.raw, pipeline);
        }
    }

    /// Bind descriptor sets to graphics pipeline.
    ///
    /// # Safety
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdBindDescriptorSets.html
    pub unsafe fn bind_graphics_descriptor_sets<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'b B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::bind_graphics_descriptor_sets(
            self.raw,
            layout,
            first_set as _,
            sets,
            offsets,
        );
    }

    /// Bind compute pipeline.
    pub fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline)
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        unsafe {
            rendy_core::hal::command::CommandBuffer::bind_compute_pipeline(self.raw, pipeline);
        }
    }

    /// Bind descriptor sets to compute pipeline.
    ///
    /// # Safety
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdBindDescriptorSets.html
    pub unsafe fn bind_compute_descriptor_sets<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'b B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    ) where
        C: Supports<Compute>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::bind_compute_descriptor_sets(
            self.raw,
            layout,
            first_set as usize,
            sets,
            offsets,
        );
    }

    /// Insert pipeline barrier.
    ///
    /// # Safety
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdPipelineBarrier.html
    pub unsafe fn pipeline_barrier<'b>(
        &mut self,
        stages: std::ops::Range<rendy_core::hal::pso::PipelineStage>,
        dependencies: rendy_core::hal::memory::Dependencies,
        barriers: impl IntoIterator<Item = rendy_core::hal::memory::Barrier<'b, B>>,
    ) {
        rendy_core::hal::command::CommandBuffer::pipeline_barrier(
            self.raw,
            stages,
            dependencies,
            barriers,
        )
    }

    /// Push graphics constants.
    ///
    /// # Safety
    ///
    /// `offset` must be multiple of 4.
    /// `constants.len() + offset`, must be less than or equal to the
    /// `maxPushConstantsSize` device limit.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdPushConstants.html
    pub unsafe fn push_constants<'b>(
        &mut self,
        layout: &B::PipelineLayout,
        stages: rendy_core::hal::pso::ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    ) {
        rendy_core::hal::command::CommandBuffer::push_graphics_constants(
            self.raw, layout, stages, offset, constants,
        );
    }

    /// Set viewports
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetViewport.html
    pub unsafe fn set_viewports<'b>(
        &mut self,
        first_viewport: u32,
        viewports: impl IntoIterator<Item = &'b rendy_core::hal::pso::Viewport>,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_viewports(self.raw, first_viewport, viewports)
    }

    /// Set scissors
    ///
    /// # Safety
    ///
    /// `first_scissor + rects.count()` must be less than the
    /// `maxViewports` device limit.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetScissor.html
    pub unsafe fn set_scissors<'b>(
        &mut self,
        first_scissor: u32,
        rects: impl IntoIterator<Item = &'b rendy_core::hal::pso::Rect>,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_scissors(self.raw, first_scissor, rects)
    }

    /// Set the stencil reference dynamic state
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetStencilReference.html
    pub unsafe fn set_stencil_reference(
        &mut self,
        faces: rendy_core::hal::pso::Face,
        value: rendy_core::hal::pso::StencilValue,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_stencil_reference(self.raw, faces, value);
    }

    /// Set the stencil compare mask dynamic state
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetStencilCompareMask.html
    pub unsafe fn set_stencil_read_mask(
        &mut self,
        faces: rendy_core::hal::pso::Face,
        value: rendy_core::hal::pso::StencilValue,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_stencil_read_mask(self.raw, faces, value);
    }

    /// Set the stencil write mask dynamic state
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetStencilWriteMask.html
    pub unsafe fn set_stencil_write_mask(
        &mut self,
        faces: rendy_core::hal::pso::Face,
        value: rendy_core::hal::pso::StencilValue,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_stencil_write_mask(self.raw, faces, value);
    }

    /// Set the values of blend constants
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetBlendConstants.html
    pub unsafe fn set_blend_constants(&mut self, color: rendy_core::hal::pso::ColorValue)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_blend_constants(self.raw, color);
    }

    /// Set the depth bounds test values
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetDepthBounds.html
    pub unsafe fn set_depth_bounds(&mut self, bounds: std::ops::Range<f32>)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_depth_bounds(self.raw, bounds);
    }

    /// Set the dynamic line width state
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetLineWidth.html
    pub unsafe fn set_line_width(&mut self, width: f32)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_line_width(self.raw, width);
    }

    /// Set the depth bias dynamic state
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdSetDepthBias.html
    pub unsafe fn set_depth_bias(&mut self, depth_bias: rendy_core::hal::pso::DepthBias)
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();
        rendy_core::hal::command::CommandBuffer::set_depth_bias(self.raw, depth_bias);
    }

    /// Reborrow encoder.
    pub fn reborrow<K>(&mut self) -> EncoderCommon<'_, B, K>
    where
        C: Supports<K>,
    {
        EncoderCommon {
            capability: self.capability.supports().unwrap(),
            raw: &mut *self.raw,
            family: self.family,
        }
    }
}

/// Special encoder to record render-pass commands.
#[derive(Debug)]
pub struct RenderPassEncoder<'a, B: rendy_core::hal::Backend> {
    inner: EncoderCommon<'a, B, Graphics>,
}

impl<'a, B> std::ops::Deref for RenderPassEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    type Target = EncoderCommon<'a, B, Graphics>;

    fn deref(&self) -> &EncoderCommon<'a, B, Graphics> {
        &self.inner
    }
}

impl<'a, B> std::ops::DerefMut for RenderPassEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    fn deref_mut(&mut self) -> &mut EncoderCommon<'a, B, Graphics> {
        &mut self.inner
    }
}

impl<'a, B> RenderPassEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    /// Clear regions within bound framebuffer attachments
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdClearAttachments.html#vkCmdBeginRenderPass
    pub unsafe fn clear_attachments(
        &mut self,
        clears: impl IntoIterator<
            Item = impl std::borrow::Borrow<rendy_core::hal::command::AttachmentClear>,
        >,
        rects: impl IntoIterator<Item = impl std::borrow::Borrow<rendy_core::hal::pso::ClearRect>>,
    ) {
        rendy_core::hal::command::CommandBuffer::clear_attachments(self.inner.raw, clears, rects);
    }

    /// Draw.
    ///
    /// # Safety
    ///
    /// The range of `vertices` must not exceed the size of the currently bound vertex buffer,
    /// and the range of `instances` must not exceed the size of the currently bound instance
    /// buffer.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdDraw.html
    pub unsafe fn draw(&mut self, vertices: std::ops::Range<u32>, instances: std::ops::Range<u32>) {
        rendy_core::hal::command::CommandBuffer::draw(self.inner.raw, vertices, instances)
    }

    /// Draw indexed, with `base_vertex` specifying an offset that is treated as
    /// vertex number 0.
    ///
    /// # Safety
    ///
    /// Same as `draw()`, plus the value of `base_vertex`.  So, `base_vertex + indices.end`
    /// must not be larger than the currently bound vertex buffer.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdDrawIndexed.html
    pub unsafe fn draw_indexed(
        &mut self,
        indices: std::ops::Range<u32>,
        base_vertex: i32,
        instances: std::ops::Range<u32>,
    ) {
        rendy_core::hal::command::CommandBuffer::draw_indexed(
            self.inner.raw,
            indices,
            base_vertex,
            instances,
        )
    }

    /// Draw indirect.
    /// Similar to [`draw`] except takes vertices and instance data from `buffer` at specified `offset`.
    /// `buffer` must contain `draw_count` of [`DrawCommand`] starting from `offset` with `stride` bytes between each.
    ///
    /// [`draw`]: trait.RenderPassInlineEncoder.html#tymethod.draw
    /// [`DrawCommand`]: struct.DrawCommand.html
    ///
    /// # Safety
    ///
    /// Similar to `draw()`.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdDrawIndirect.html
    pub unsafe fn draw_indirect(
        &mut self,
        buffer: &B::Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        rendy_core::hal::command::CommandBuffer::draw_indirect(
            self.inner.raw,
            buffer,
            offset,
            draw_count,
            stride,
        )
    }

    /// Draw indirect with indices.
    /// Similar to [`draw_indexed`] except takes vertices, indices and instance data from `buffer` at specified `offset`.
    /// `buffer` must contain `draw_count` of [`DrawIndexedCommand`] starting from `offset` with `stride` bytes between each.
    ///
    /// [`draw`]: trait.RenderPassInlineEncoder.html#tymethod.draw_indexed
    /// [`DrawIndexedCommand`]: struct.DrawIndexedCommand.html
    ///
    /// # Safety
    ///
    /// Similar to `draw_indexed()`
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdDrawIndexedIndirect.html
    pub unsafe fn draw_indexed_indirect(
        &mut self,
        buffer: &B::Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        rendy_core::hal::command::CommandBuffer::draw_indexed_indirect(
            self.inner.raw,
            buffer,
            offset,
            draw_count,
            stride,
        )
    }

    /// Reborrow encoder.
    pub fn reborrow(&mut self) -> RenderPassEncoder<'_, B> {
        RenderPassEncoder {
            inner: self.inner.reborrow(),
        }
    }
}

/// Special encoder to record commands inside render pass.
#[derive(Debug)]
pub struct RenderPassInlineEncoder<'a, B: rendy_core::hal::Backend> {
    inner: RenderPassEncoder<'a, B>,
}

impl<'a, B> Drop for RenderPassInlineEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    fn drop(&mut self) {
        unsafe { rendy_core::hal::command::CommandBuffer::end_render_pass(self.inner.inner.raw) }
    }
}

impl<'a, B> std::ops::Deref for RenderPassInlineEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    type Target = RenderPassEncoder<'a, B>;

    fn deref(&self) -> &RenderPassEncoder<'a, B> {
        &self.inner
    }
}

impl<'a, B> std::ops::DerefMut for RenderPassInlineEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    fn deref_mut(&mut self) -> &mut RenderPassEncoder<'a, B> {
        &mut self.inner
    }
}

impl<'a, B> RenderPassInlineEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    /// Record next subpass inline.
    pub fn next_subpass_inline(self) -> RenderPassInlineEncoder<'a, B> {
        unsafe {
            rendy_core::hal::command::CommandBuffer::next_subpass(
                self.inner.inner.raw,
                rendy_core::hal::command::SubpassContents::Inline,
            );
        }

        self
    }

    /// Record next subpass secondary.
    pub fn next_subpass_secondary(self) -> RenderPassSecondaryEncoder<'a, B> {
        unsafe {
            rendy_core::hal::command::CommandBuffer::next_subpass(
                self.inner.inner.raw,
                rendy_core::hal::command::SubpassContents::SecondaryBuffers,
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
pub struct RenderPassSecondaryEncoder<'a, B: rendy_core::hal::Backend> {
    inner: EncoderCommon<'a, B, Graphics>,
}

impl<'a, B> Drop for RenderPassSecondaryEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    fn drop(&mut self) {
        unsafe { rendy_core::hal::command::CommandBuffer::end_render_pass(self.inner.raw) }
    }
}

impl<'a, B> RenderPassSecondaryEncoder<'a, B>
where
    B: rendy_core::hal::Backend,
{
    /// Execute commands from secondary buffers.
    pub fn execute_commands(
        &mut self,
        submittables: impl IntoIterator<Item = impl Submittable<B, SecondaryLevel, RenderPassContinue>>,
    ) {
        let family = self.inner.family;
        unsafe {
            rendy_core::hal::command::CommandBuffer::execute_commands(
                self.inner.raw,
                submittables.into_iter().map(|submit| {
                    assert_eq!(family, submit.family());
                    submit.raw()
                }),
            )
        }
    }

    /// Record next subpass inline.
    pub fn next_subpass_inline(self) -> RenderPassInlineEncoder<'a, B> {
        unsafe {
            rendy_core::hal::command::CommandBuffer::next_subpass(
                self.inner.raw,
                rendy_core::hal::command::SubpassContents::Inline,
            );

            let next = RenderPassInlineEncoder {
                inner: RenderPassEncoder {
                    inner: std::ptr::read(&self.inner),
                },
            };

            std::mem::forget(self);
            next
        }
    }

    /// Record next subpass secondary.
    pub fn next_subpass_secondary(self) -> RenderPassSecondaryEncoder<'a, B> {
        unsafe {
            rendy_core::hal::command::CommandBuffer::next_subpass(
                self.inner.raw,
                rendy_core::hal::command::SubpassContents::SecondaryBuffers,
            );
        }

        self
    }
}

/// Trait to encode commands outside render pass.
#[derive(Debug)]
pub struct Encoder<'a, B: rendy_core::hal::Backend, C, L> {
    inner: EncoderCommon<'a, B, C>,
    level: L,
}

impl<'a, B, C, L> std::ops::Deref for Encoder<'a, B, C, L>
where
    B: rendy_core::hal::Backend,
{
    type Target = EncoderCommon<'a, B, C>;

    fn deref(&self) -> &EncoderCommon<'a, B, C> {
        &self.inner
    }
}

impl<'a, B, C, L> std::ops::DerefMut for Encoder<'a, B, C, L>
where
    B: rendy_core::hal::Backend,
{
    fn deref_mut(&mut self) -> &mut EncoderCommon<'a, B, C> {
        &mut self.inner
    }
}

impl<'a, B, C> Encoder<'a, B, C, PrimaryLevel>
where
    B: rendy_core::hal::Backend,
{
    /// Beging recording render pass inline.
    pub fn begin_render_pass_inline(
        &mut self,
        render_pass: &B::RenderPass,
        framebuffer: &B::Framebuffer,
        render_area: rendy_core::hal::pso::Rect,
        clear_values: &[rendy_core::hal::command::ClearValue],
    ) -> RenderPassInlineEncoder<'_, B>
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            rendy_core::hal::command::CommandBuffer::begin_render_pass(
                self.inner.raw,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                rendy_core::hal::command::SubpassContents::Inline,
            )
        }

        RenderPassInlineEncoder {
            inner: RenderPassEncoder {
                inner: self.inner.reborrow(),
            },
        }
    }

    /// Beging recording render pass secondary.
    pub fn begin_render_pass_secondary(
        &mut self,
        render_pass: &B::RenderPass,
        framebuffer: &B::Framebuffer,
        render_area: rendy_core::hal::pso::Rect,
        clear_values: &[rendy_core::hal::command::ClearValue],
    ) -> RenderPassSecondaryEncoder<'_, B>
    where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        unsafe {
            rendy_core::hal::command::CommandBuffer::begin_render_pass(
                self.inner.raw,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                rendy_core::hal::command::SubpassContents::SecondaryBuffers,
            )
        }

        RenderPassSecondaryEncoder {
            inner: self.inner.reborrow(),
        }
    }

    /// Execute commands from secondary buffers.
    pub fn execute_commands(
        &mut self,
        submittables: impl IntoIterator<Item = impl Submittable<B, SecondaryLevel>>,
    ) {
        let family = self.inner.family;
        unsafe {
            rendy_core::hal::command::CommandBuffer::execute_commands(
                self.inner.raw,
                submittables.into_iter().map(|submit| {
                    assert_eq!(family, submit.family());
                    submit.raw()
                }),
            )
        }
    }
}

impl<'a, B, C, L> Encoder<'a, B, C, L>
where
    B: rendy_core::hal::Backend,
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
    ///
    /// # Safety
    ///
    /// The size of the copy region in any `regions` must not exceed the
    /// length of the corresponding buffer.
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdCopyBuffer.html
    pub unsafe fn copy_buffer(
        &mut self,
        src: &B::Buffer,
        dst: &B::Buffer,
        regions: impl IntoIterator<Item = rendy_core::hal::command::BufferCopy>,
    ) where
        C: Supports<Transfer>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::copy_buffer(self.inner.raw, src, dst, regions)
    }

    /// Copy buffer region to image subresource range.
    ///
    /// # Safety
    ///
    /// Same as `copy_buffer()`
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdCopyBufferToImage.html
    pub unsafe fn copy_buffer_to_image(
        &mut self,
        src: &B::Buffer,
        dst: &B::Image,
        dst_layout: rendy_core::hal::image::Layout,
        regions: impl IntoIterator<Item = rendy_core::hal::command::BufferImageCopy>,
    ) where
        C: Supports<Transfer>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::copy_buffer_to_image(
            self.inner.raw,
            src,
            dst,
            dst_layout,
            regions,
        )
    }

    /// Copy image regions.
    ///
    /// # Safety
    ///
    /// Same as `copy_buffer()`
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdCopyImage.html
    pub unsafe fn copy_image(
        &mut self,
        src: &B::Image,
        src_layout: rendy_core::hal::image::Layout,
        dst: &B::Image,
        dst_layout: rendy_core::hal::image::Layout,
        regions: impl IntoIterator<Item = rendy_core::hal::command::ImageCopy>,
    ) where
        C: Supports<Transfer>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::copy_image(
            self.inner.raw,
            src,
            src_layout,
            dst,
            dst_layout,
            regions,
        )
    }

    /// Copy image subresource range to buffer region.
    ///
    /// # Safety
    ///
    /// Same as `copy_buffer()`
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdCopyImageToBuffer.html
    pub unsafe fn copy_image_to_buffer(
        &mut self,
        src: &B::Image,
        src_layout: rendy_core::hal::image::Layout,
        dst: &B::Buffer,
        regions: impl IntoIterator<Item = rendy_core::hal::command::BufferImageCopy>,
    ) where
        C: Supports<Transfer>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::copy_image_to_buffer(
            self.inner.raw,
            src,
            src_layout,
            dst,
            regions,
        )
    }

    /// Blit image regions, potentially using specified filter when resize is necessary.
    ///
    /// # Safety
    ///
    /// Same as `copy_buffer()`
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdBlitImage.html
    pub unsafe fn blit_image(
        &mut self,
        src: &B::Image,
        src_layout: rendy_core::hal::image::Layout,
        dst: &B::Image,
        dst_layout: rendy_core::hal::image::Layout,
        filter: rendy_core::hal::image::Filter,
        regions: impl IntoIterator<Item = rendy_core::hal::command::ImageBlit>,
    ) where
        C: Supports<Graphics>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::blit_image(
            self.inner.raw,
            src,
            src_layout,
            dst,
            dst_layout,
            filter,
            regions,
        )
    }

    /// Dispatch compute.
    ///
    /// # Safety
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdDispatch.html
    pub unsafe fn dispatch(&mut self, x: u32, y: u32, z: u32)
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::dispatch(self.inner.raw, [x, y, z])
    }

    /// Dispatch indirect.
    /// Similar to [`dispatch`] except takes vertices and indices from `buffer` at specified `offset`.
    /// `buffer` must contain [`DispatchCommand`] at `offset`.
    ///
    /// [`dispatch`]: trait.Encoder.html#tymethod.dispatch
    /// [`DispatchCommand`]: struct.DispatchCommand.html
    ///
    /// # Safety
    ///
    /// See: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCmdDispatchIndirect.html
    pub unsafe fn dispatch_indirect(&mut self, buffer: &B::Buffer, offset: u64)
    where
        C: Supports<Compute>,
    {
        self.capability.assert();

        rendy_core::hal::command::CommandBuffer::dispatch_indirect(self.inner.raw, buffer, offset)
    }
}

impl<B, C, U, L, R> CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: rendy_core::hal::Backend,
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
    B: rendy_core::hal::Backend,
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
