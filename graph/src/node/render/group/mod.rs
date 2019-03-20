mod simple;

pub use self::simple::*;

use {
    crate::{
        command::{QueueId, RenderPassEncoder},
        factory::Factory,
        node::{
            render::{pass::SubpassBuilder, PrepareResult},
            BufferAccess, DescBuilder, ImageAccess, NodeBuffer, NodeImage,
        },
        BufferId, ImageId, NodeId,
    },
    gfx_hal::Backend,
};

pub trait RenderGroupDesc<B: Backend, T: ?Sized>: std::fmt::Debug {
    /// Make render group builder.
    fn builder(self) -> DescBuilder<B, T, Self>
    where
        Self: Sized,
    {
        DescBuilder {
            desc: self,
            buffers: Vec::new(),
            images: Vec::new(),
            dependencies: Vec::new(),
            marker: std::marker::PhantomData,
        }
    }

    /// Get buffers used by the group
    fn buffers(&self) -> Vec<BufferAccess>;

    /// Get images used by the group
    fn images(&self) -> Vec<ImageAccess>;

    /// Number of color output images.
    fn colors(&self) -> usize;

    /// Is depth image used.
    fn depth(&self) -> bool;

    /// Build render group.
    fn build<'a>(
        self,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &T,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: gfx_hal::pass::Subpass<'_, B>,
        buffers: Vec<NodeBuffer<'a, B>>,
        images: Vec<NodeImage<'a, B>>,
    ) -> Result<Box<dyn RenderGroup<B, T>>, failure::Error>;
}

pub trait RenderGroup<B: Backend, T: ?Sized>: std::fmt::Debug + Send + Sync {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        queue: QueueId,
        index: usize,
        subpass: gfx_hal::pass::Subpass<'_, B>,
        aux: &T,
    ) -> PrepareResult;

    fn draw_inline(
        &mut self,
        encoder: RenderPassEncoder<'_, B>,
        index: usize,
        subpass: gfx_hal::pass::Subpass<'_, B>,
        aux: &T,
    );

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &T);
}

pub trait RenderGroupBuilder<B: Backend, T: ?Sized>: std::fmt::Debug {
    /// Make subpass from render group.
    fn into_subpass(self) -> SubpassBuilder<B, T>
    where
        Self: Sized + 'static,
    {
        SubpassBuilder::new().with_group(self)
    }

    /// Number of color output images.
    fn colors(&self) -> usize;

    /// Is depth image used.
    fn depth(&self) -> bool;

    /// Get buffers used by the group
    fn buffers(&self) -> Vec<(BufferId, BufferAccess)>;

    /// Get images used by the group
    fn images(&self) -> Vec<(ImageId, ImageAccess)>;

    /// Get nodes this group depends on.
    fn dependencies(&self) -> Vec<NodeId>;

    fn build<'a>(
        self: Box<Self>,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &T,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: gfx_hal::pass::Subpass<'_, B>,
        buffers: Vec<NodeBuffer<'a, B>>,
        images: Vec<NodeImage<'a, B>>,
    ) -> Result<Box<dyn RenderGroup<B, T>>, failure::Error>;
}

impl<B, T, D> RenderGroupBuilder<B, T> for DescBuilder<B, T, D>
where
    B: Backend,
    T: ?Sized,
    D: RenderGroupDesc<B, T>,
{
    fn colors(&self) -> usize {
        self.desc.colors()
    }

    fn depth(&self) -> bool {
        self.desc.depth()
    }

    fn buffers(&self) -> Vec<(BufferId, BufferAccess)> {
        self.buffers
            .iter()
            .cloned()
            .zip(self.desc.buffers())
            .collect()
    }

    fn images(&self) -> Vec<(ImageId, ImageAccess)> {
        self.images
            .iter()
            .cloned()
            .zip(self.desc.images())
            .collect()
    }

    fn dependencies(&self) -> Vec<NodeId> {
        self.dependencies.clone()
    }

    fn build<'a>(
        self: Box<Self>,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &T,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: gfx_hal::pass::Subpass<'_, B>,
        buffers: Vec<NodeBuffer<'a, B>>,
        images: Vec<NodeImage<'a, B>>,
    ) -> Result<Box<dyn RenderGroup<B, T>>, failure::Error> {
        self.desc.build(
            factory,
            queue,
            aux,
            framebuffer_width,
            framebuffer_height,
            subpass,
            buffers,
            images,
        )
    }
}
