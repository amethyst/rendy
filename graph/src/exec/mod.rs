use std::marker::PhantomData;
use std::sync::Arc;
use std::ops::Range;

use crate::factory::Factory;

use crate::core::hal;
use crate::resource::Image;
use crate::scheduler::interface::{ImageToken, SemaphoreId};
use crate::shader::ShaderId;

use crate::command2::{RenderPassId, HashableGraphicsPipelineDescr, HashablePrimitiveAssemblerDescr, CacheGraphicsPipelineTypes, Cache};

use hal::command::CommandBuffer;

use crate::{GraphicsPipelineBuilder, PrimitiveAssemblerKind};

#[derive(Debug, Clone)]
pub(crate) struct SubpassData {
    pub(crate) render_pass: RenderPassId,
    pub(crate) subpass_idx: u8,
}

pub struct ExecCtx<'a, B: hal::Backend> {
    pub(crate) phantom: PhantomData<B>,

    pub(crate) factory: &'a Factory<B>,
    pub(crate) cache: Arc<Cache<B>>,

    pub(crate) active_subpass: Option<SubpassData>,

    pub(crate) command_buffer: B::CommandBuffer,
}

impl<'a, B: hal::Backend> ExecCtx<'a, B> {

    /// Return the given semaphore to the render graphs internal pool.
    ///
    /// The render graph will make sure to only reuse the semaphore after
    /// the currently executing graph has finished executing.
    pub fn return_semaphore(&mut self, semaphore: B::Semaphore) {
        todo!()
    }

    pub fn get_image(&self) -> &Image<B> {
        todo!()
    }

    /// Fetches a provided image back from the graph.
    ///
    /// This can only be called if:
    /// * `ImageId` is a provided image of the regular image type
    /// * The current entity is the last use of `ImageId`
    ///
    /// If any of these are not true, it will panic.
    pub fn fetch_image(&mut self, image_token: ImageToken) -> Image<B> {
        todo!()
    }

    /// Fetches a provided swapchain image back from the graph.
    ///
    /// This can only be called if:
    /// * `ImageId` is a provided image of the swapchain image type
    /// * The current entity is the last use of `ImageId`
    ///
    /// If any of these are not true, it will panic.
    pub fn fetch_swapchain_image(&mut self, image_token: ImageToken) -> <B::Surface as hal::window::PresentationSurface<B>>::SwapchainImage {
        todo!()
    }

    /// Fetches a semaphore by its construction id.
    ///
    /// This will return None if no synchronization is required.
    pub fn fetch_semaphore(&mut self, semaphore_id: SemaphoreId) -> Option<B::Semaphore> {
        todo!()
    }

    pub fn bind_graphics_pipeline(&mut self, shader_set: ShaderId, descr: GraphicsPipelineBuilder) {
        let subpass = self.active_subpass.clone().unwrap();

        let key: HashableGraphicsPipelineDescr<CacheGraphicsPipelineTypes> = HashableGraphicsPipelineDescr {
            label: descr.label,

            program: shader_set,
            subpass: (subpass.render_pass, subpass.subpass_idx),

            primitive_assembler: match descr.primitive_assembler_kind {
                PrimitiveAssemblerKind::Vertex => HashablePrimitiveAssemblerDescr::Vertex {
                    input_assembler: descr.input_assembler,
                    tessellation: descr.tessellation,
                    geometry: descr.geometry,
                },
                PrimitiveAssemblerKind::Mesh => HashablePrimitiveAssemblerDescr::Mesh {
                    task: descr.task,
                },
            },
            rasterizer: descr.rasterizer,
            fragment: descr.fragment,
            blender: descr.blender,
            depth_stencil: descr.depth_stencil,
            multisampling: descr.multisampling,
            //baked_states: {
            //    let bs = descr.baked_states;
            //    hal::pso::BakedStates {
            //        viewport: match bs.viewport {
            //            MaybeInfer::None => None,
            //            MaybeInfer::Infer => Some(subpass.viewport),
            //            MaybeInfer::Some(viewport) => Some(viewport),
            //        },
            //        scissor: match bs.scissor {
            //            MaybeInfer::None => None,
            //            MaybeInfer::Infer => Some(subpass.viewport.rect),
            //            MaybeInfer::Some(rect) => Some(rect),
            //        },
            //        blend_color: bs.blend_color,
            //        depth_bounds: bs.depth_bounds,
            //    }
            //},
        };

        let key_arc = Arc::new(key);

        let graphics_pipeline_id = self.cache.make_graphics_pipeline(self.factory, key_arc);

        unsafe {
            self.command_buffer.bind_graphics_pipeline(&self.cache.get_graphics_pipeline(graphics_pipeline_id));
        }
    }

    pub fn bind_vertex_buffers<'b, T>(&mut self, first_index: u32, buffers: T)
    where
        T: Iterator<Item = (&'b B::Buffer, hal::buffer::SubRange)>,
    {
        unsafe {
            self.command_buffer.bind_vertex_buffers(first_index, buffers)
        }
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        unsafe {
            self.command_buffer.draw(vertices, instances)
        }
    }

    // TODO validation
    pub fn set_viewports<I>(&mut self, first_viewport: u32, viewports: I)
    where
        I: Iterator<Item = hal::pso::Viewport>,
    {
        unsafe {
            self.command_buffer.set_viewports(first_viewport, viewports);
        }
    }

    // TODO validation
    pub fn set_scissors<I>(&mut self, first_scissor: u32, rects: I)
    where
        I: Iterator<Item = hal::pso::Rect>,
    {
        unsafe {
            self.command_buffer.set_scissors(first_scissor, rects);
        }
    }

}
