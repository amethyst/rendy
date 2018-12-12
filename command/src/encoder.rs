
use crate::{
    capability::{Supports, Graphics, Transfer, Compute},
    buffer::Submittable,
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

/// Trait to encode commands.
pub trait EncoderCommon<B: gfx_hal::Backend, C> {
    /// Bind index buffer.
    fn bind_index_buffer<'b>(&mut self, buffer: &'b B::Buffer, offset: u64, index_type: gfx_hal::IndexType)
    where
        C: Supports<Graphics>,
    ;

    /// Bind vertex buffers.
    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b B::Buffer, u64)>)
    where
        C: Supports<Graphics>,
    ;

    /// Bind graphics pipeline.
    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline)
    where
        C: Supports<Graphics>,
    ;

    /// Bind descriptor sets to graphics pipeline.
    fn bind_graphics_descriptor_sets<'a>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'a B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    )
    where
        C: Supports<Graphics>,
    ;

    /// Bind graphics pipeline.
    fn bind_compute_pipeline(&mut self, pipeline: &B::ComputePipeline)
    where
        C: Supports<Compute>,
    ;

    /// Bind descriptor sets to compute pipeline.
    fn bind_compute_descriptor_sets<'a>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: u32,
        sets: impl IntoIterator<Item = &'a B::DescriptorSet>,
        offsets: impl IntoIterator<Item = u32>,
    )
    where
        C: Supports<Compute>,
    ;

	/// Insert pipeline barrier.
	fn pipeline_barrier<'a>(
		&mut self,
        stages: std::ops::Range<gfx_hal::pso::PipelineStage>,
        dependencies: gfx_hal::memory::Dependencies,
        barriers: impl IntoIterator<Item = gfx_hal::memory::Barrier<'a, B>>,
	);

    /// Execute commands from secondary buffers.
	fn execute_commands(&mut self, submittables: impl IntoIterator<Item = impl Submittable<B>>);
}

/// Trait to encode commands inside render pass.
pub trait RenderPassEncoder<B: gfx_hal::Backend>: EncoderCommon<B, Graphics> {
    /// Draw.
    fn draw(
        &mut self, 
        vertices: std::ops::Range<u32>, 
        instances: std::ops::Range<u32>,
    );

    /// Draw indexed.
    fn draw_indexed(
        &mut self, 
        indices: std::ops::Range<u32>, 
        base_vertex: i32, 
        instances: std::ops::Range<u32>,
    );

    /// Draw indirect.
    /// Similar to [`draw`] except takes vertices and indices from `buffer` at specified `offset`.
    /// `buffer` must contain `draw_count` of [`DrawCommand`] starting from `offset` with `stride` bytes between each.
    /// 
    /// [`draw`]: trait.RenderPassEncoder.html#tymethod.draw
    /// [`DrawCommand`]: struct.DrawCommand.html
    fn draw_indirect(
        &mut self, 
        buffer: &B::Buffer, 
        offset: u64, 
        draw_count: u32, 
        stride: u32,
    );
}

/// HRTB workaround.
pub trait RenderPassEncoderHRTB<'a, B: gfx_hal::Backend, C> {

    /// Render pass encoder.
    type RenderPassEncoder: RenderPassEncoder<B>;
}

/// Trait to encode commands outside render pass.
pub trait Encoder<B: gfx_hal::Backend, C>: EncoderCommon<B, C> + for<'a> RenderPassEncoderHRTB<'a, B, C> {

    /// Beging recording render pass.
    fn begin_render_pass_inline<'a>(
        &'a mut self,
        render_pass: &B::RenderPass, 
        framebuffer: &B::Framebuffer, 
        render_area: gfx_hal::pso::Rect, 
        clear_values: &[gfx_hal::command::ClearValueRaw],
    ) -> <Self as RenderPassEncoderHRTB<'a, B, C>>::RenderPassEncoder
    where
        C: Supports<Graphics>,
    ;

    /// Copy image regions.
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
    ;

    /// Dispatch compute.
    fn dispatch(&mut self, x: u32, y: u32, z: u32)
    where
        C: Supports<Compute>,
    ;

    /// Dispatch indirect.
    /// Similar to [`dispatch`] except takes vertices and indices from `buffer` at specified `offset`.
    /// `buffer` must contain [`DispatchCommand`] at `offset`.
    /// 
    /// [`dispatch`]: trait.Encoder.html#tymethod.dispatch
    /// [`DispatchCommand`]: struct.DispatchCommand.html
    fn dispatch_indirect(&mut self, buffer: &B::Buffer, offset: u64)
    where
        C: Supports<Compute>,
    ;
}
