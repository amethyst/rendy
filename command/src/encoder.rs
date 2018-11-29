
use crate::{
    capability::{Supports, Graphics, Transfer},
    resource::{Buffer, Image},
};

/// Trait to encode commands.
pub trait EncoderCommon<B: gfx_hal::Backend, C> {
    /// Bind index buffer.
    fn bind_index_buffer<'b>(&mut self, buffer: &'b Buffer<B>, offset: u64, index_type: gfx_hal::IndexType)
    where
        C: Supports<Graphics>,
    ;

    /// Bind vertex buffers.
    fn bind_vertex_buffers<'b>(&mut self, first_binding: u32, buffers: impl IntoIterator<Item = (&'b Buffer<B>, u64)>)
    where
        C: Supports<Graphics>,
    ;

    /// Bind graphics pipeline.
    fn bind_graphics_pipeline(&mut self, pipeline: &B::GraphicsPipeline)
    where
        C: Supports<Graphics>,
    ;
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
}
