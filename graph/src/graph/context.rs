use std::any::Any;
use std::sync::Arc;

use rendy_core::hal;

use crate::exec::ExecCtx;
use crate::factory::Factory;
use crate::command::{Queue, RenderPassEncoder};
use crate::scheduler::{
    builder::ProceduralBuilder,
    interface::{
        EntityConstructionError,
        GraphCtx, EntityCtx, PassEntityCtx,
        Root, ImageToken, BufferToken,
        ImageId, BufferId, EntityId, FenceId, SemaphoreId, VirtualId,
        PersistentKind, PersistenceToken,
    },
    sync::{SyncPoint, HasSyncPoint, SyncPointRef},
    resources::{
        ImageInfo, BufferInfo,
        ImageUsage, BufferUsage,
        VirtualUsage,
        ProvidedImageUsage, ProvidedBufferUsage,
    },
};

use crate::GraphBorrow;
use super::{Graph, GfxSchedulerTypes, GraphImage};

pub struct GraphConstructCtx<'a, 'b, B: hal::Backend> {
    pub(crate) inner: &'a mut Graph<'b, B>,
    pub(crate) node_id: super::GraphGenerationNodeId,
}
impl<'a, 'b, B: hal::Backend> GraphConstructCtx<'a, 'b, B> {
    pub fn pass<'c>(&'c mut self) -> PassConstructCtx<'c, 'b, B> {
        self.inner.procedural.start_pass();
        PassConstructCtx {
            inner: self.inner,
            node_id: self.node_id,
            relevant: relevant::Relevant,
        }
    }
    pub fn standalone<'c>(&'c mut self) -> StandaloneConstructCtx<'c, 'b, B> {
        self.inner.procedural.start_standalone();
        StandaloneConstructCtx {
            inner: self.inner,
            node_id: self.node_id,
            relevant: relevant::Relevant,
        }
    }
    pub fn present<F>(
        &mut self,
        surface: GraphBorrow<rendy_wsi::Surface<B>>,
        image: ImageId,
        result_handler: F,
    )
    where
        F: FnOnce(&mut dyn Any, Result<Option<hal::window::Suboptimal>, hal::window::PresentError>) + 'static,
    {
        self.inner.add_present(surface, image, result_handler)
    }
}

pub struct PassConstructCtx<'a, 'b, B: hal::Backend> {
    pub(crate) inner: &'a mut Graph<'b, B>,
    pub(crate) node_id: super::GraphGenerationNodeId,
    relevant: relevant::Relevant,
}
impl<'a, 'b, B: hal::Backend> PassConstructCtx<'a, 'b, B> {
    pub fn commit<F: FnOnce(&mut dyn Any, &Arc<B::Device>, &mut ExecCtx<B>, &mut RenderPassEncoder<B>)>(self, _exec: F) {
        todo!();
        //self.inner.commit(());
        self.relevant.dispose();
    }
}
impl<'a, 'b, B: hal::Backend> PassEntityCtx<GfxSchedulerTypes<B>> for PassConstructCtx<'a, 'b, B> {
    fn use_color(
        &mut self,
        index: usize,
        image: ImageId,
        read_access: bool,
    ) -> Result<(), EntityConstructionError> {
        self.inner.procedural.use_color(index, image, read_access)
    }
    fn use_depth(
        &mut self,
        image: ImageId,
        write_access: bool,
    ) -> Result<(), EntityConstructionError> {
        self.inner.procedural.use_depth(image, write_access)
    }
    fn use_input(
        &mut self,
        index: usize,
        image: ImageId,
    ) -> Result<(), EntityConstructionError> {
        self.inner.procedural.use_input(index, image)
    }
}

pub struct StandaloneConstructCtx<'a, 'b, B: hal::Backend> {
    pub(crate) inner: &'a mut Graph<'b, B>,
    pub(crate) node_id: super::GraphGenerationNodeId,
    relevant: relevant::Relevant,
}
impl<'a, 'b, B: hal::Backend> StandaloneConstructCtx<'a, 'b, B> {
    pub fn commit<F>(self, exec: F)
    where
        F: FnOnce(&mut dyn Any, &Factory<B>, &mut ExecCtx<B>, &mut Queue<B>) + 'static,
    {
        self.inner.commit_standalone(self.node_id, exec);
        self.relevant.dispose();
    }
}
