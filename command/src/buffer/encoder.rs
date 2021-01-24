use derive_more::{Deref, DerefMut};
use rendy_core::hal;

use super::{
    level::{Level, PrimaryLevel, SecondaryLevel},
    state::RecordingState,
    submit::Submittable,
    usage::RenderPassContinue,
    CommandBuffer,
};
use crate::{
    capability::{Capability, Compute, Graphics, Supports},
    family::FamilyId,
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
#[derive(Debug, Deref, DerefMut)]
pub struct EncoderCommon<'a, B: hal::Backend, C> {
    #[deref]
    #[deref_mut]
    raw: &'a mut B::CommandBuffer,
    capability: C,
    family: FamilyId,
}

impl<'a, B, C> EncoderCommon<'a, B, C>
where
    B: hal::Backend,
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
    pub unsafe fn bind_index_buffer(
        &mut self,
        buffer: &B::Buffer,
        offset: u64,
        index_type: hal::IndexType,
    ) where
        C: Supports<Graphics>,
    {
        hal::command::CommandBuffer::bind_index_buffer(
            self.raw,
            hal::buffer::IndexBufferView {
                buffer,
                range: hal::buffer::SubRange { offset, size: None },
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
    pub unsafe fn bind_vertex_buffers<'b, I>(&mut self, first_binding: u32, buffers: I)
    where
        I: IntoIterator<Item = (&'b B::Buffer, u64)>,
        I::IntoIter: ExactSizeIterator,
        C: Supports<Graphics>,
    {
        hal::command::CommandBuffer::bind_vertex_buffers(
            self.raw,
            first_binding,
            buffers
                .into_iter()
                .map(|(buffer, offset)| (buffer, hal::buffer::SubRange { offset, size: None })),
        )
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
#[derive(Debug, Deref, DerefMut)]
pub struct RenderPassEncoder<'a, B: hal::Backend> {
    inner: EncoderCommon<'a, B, Graphics>,
}

impl<'a, B> RenderPassEncoder<'a, B>
where
    B: hal::Backend,
{
    /// Reborrow encoder.
    pub fn reborrow(&mut self) -> RenderPassEncoder<'_, B> {
        RenderPassEncoder {
            inner: self.inner.reborrow(),
        }
    }
}

/// Special encoder to record commands inside render pass.
#[derive(Debug, Deref, DerefMut)]
pub struct RenderPassInlineEncoder<'a, B: hal::Backend> {
    inner: RenderPassEncoder<'a, B>,
}

impl<'a, B> Drop for RenderPassInlineEncoder<'a, B>
where
    B: hal::Backend,
{
    fn drop(&mut self) {
        unsafe { hal::command::CommandBuffer::end_render_pass(self.inner.inner.raw) }
    }
}

impl<'a, B> RenderPassInlineEncoder<'a, B>
where
    B: hal::Backend,
{
    /// Record next subpass inline.
    pub fn next_subpass_inline(self) -> RenderPassInlineEncoder<'a, B> {
        unsafe {
            hal::command::CommandBuffer::next_subpass(
                self.inner.inner.raw,
                hal::command::SubpassContents::Inline,
            );
        }

        self
    }

    /// Record next subpass secondary.
    pub fn next_subpass_secondary(self) -> RenderPassSecondaryEncoder<'a, B> {
        unsafe {
            hal::command::CommandBuffer::next_subpass(
                self.inner.inner.raw,
                hal::command::SubpassContents::SecondaryBuffers,
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
pub struct RenderPassSecondaryEncoder<'a, B: hal::Backend> {
    inner: EncoderCommon<'a, B, Graphics>,
}

impl<'a, B> Drop for RenderPassSecondaryEncoder<'a, B>
where
    B: hal::Backend,
{
    fn drop(&mut self) {
        unsafe { hal::command::CommandBuffer::end_render_pass(self.inner.raw) }
    }
}

impl<'a, B> RenderPassSecondaryEncoder<'a, B>
where
    B: hal::Backend,
{
    /// Execute commands from secondary buffers.
    pub fn execute_commands<I>(&mut self, submittables: I)
    where
        I: IntoIterator,
        I::Item: Submittable<B, SecondaryLevel, RenderPassContinue>,
        I::IntoIter: ExactSizeIterator,
    {
        unsafe {
            hal::command::CommandBuffer::execute_commands(
                self.inner.raw,
                submittables.into_iter().map(|submit| submit.raw()),
            )
        }
    }

    /// Record next subpass inline.
    pub fn next_subpass_inline(self) -> RenderPassInlineEncoder<'a, B> {
        unsafe {
            hal::command::CommandBuffer::next_subpass(
                self.inner.raw,
                hal::command::SubpassContents::Inline,
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
            hal::command::CommandBuffer::next_subpass(
                self.inner.raw,
                hal::command::SubpassContents::SecondaryBuffers,
            );
        }

        self
    }
}

/// Trait to encode commands outside render pass.
#[derive(Debug, Deref, DerefMut)]
pub struct Encoder<'a, B: hal::Backend, C, L> {
    #[deref]
    #[deref_mut]
    inner: EncoderCommon<'a, B, C>,
    level: L,
}

impl<'a, B, C> Encoder<'a, B, C, PrimaryLevel>
where
    B: hal::Backend,
{
    /// Beging recording render pass inline.
    pub fn begin_render_pass_inline(
        &mut self,
        render_pass: &B::RenderPass,
        framebuffer: &B::Framebuffer,
        render_area: hal::pso::Rect,
        clear_values: &[hal::command::ClearValue],
    ) -> RenderPassInlineEncoder<'_, B>
    where
        C: Supports<Graphics>,
    {
        unsafe {
            hal::command::CommandBuffer::begin_render_pass(
                self.inner.raw,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                hal::command::SubpassContents::Inline,
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
        render_area: hal::pso::Rect,
        clear_values: &[hal::command::ClearValue],
    ) -> RenderPassSecondaryEncoder<'_, B>
    where
        C: Supports<Graphics>,
    {
        unsafe {
            hal::command::CommandBuffer::begin_render_pass(
                self.inner.raw,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                hal::command::SubpassContents::SecondaryBuffers,
            )
        }

        RenderPassSecondaryEncoder {
            inner: self.inner.reborrow(),
        }
    }

    /// Execute commands from secondary buffers.
    pub fn execute_commands<I>(&mut self, submittables: I)
    where
        I: IntoIterator,
        I::Item: Submittable<B, SecondaryLevel>,
        I::IntoIter: ExactSizeIterator,
    {
        unsafe {
            hal::command::CommandBuffer::execute_commands(
                self.inner.raw,
                submittables.into_iter().map(|submit| submit.raw()),
            )
        }
    }
}

impl<'a, B, C, L> Encoder<'a, B, C, L>
where
    B: hal::Backend,
{
    /// Get encoder level.
    pub fn level(&self) -> L
    where
        L: Level,
    {
        self.level
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
        hal::command::CommandBuffer::dispatch(self.inner.raw, [x, y, z])
    }
}

impl<B, C, U, L, R> CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: hal::Backend,
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
    B: hal::Backend,
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
