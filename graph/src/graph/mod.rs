use std::alloc::Allocator;
use std::any::Any;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryInto;
use std::marker::PhantomData;
use std::sync::Arc;

use hal::command::CommandBuffer as _;
use hal::device::Device as _;
use hal::queue::CommandQueue as _;
use hal::window::PresentationSurface;
use rendy_core::hal;

use cranelift_entity::{entity_impl, PrimaryMap, SecondaryMap};

use crate::SliceBuf;

use crate::Frames;
use crate::command::RenderPassEncoder;
use crate::command::{CommandBuffer, Queue};
use crate::command2::{Cache, RenderPassId, RenderPassKey, ShaderSetKey, SubpassDescKey};
use crate::exec::{ExecCtx, SubpassData};
use crate::factory::Factory;
use crate::scheduler::{
    input::ResourceId,
    interface::{EntityCtx, EntityId, GraphCtx, ImageId, SemaphoreId},
    procedural::{ImageSource, ProceduralBuilder, ResourceKind},
    resources::ImageUsage,
    schedule_iterator::{Current, ScheduleIterator},
    ExternalSignal, RenderPass, ScheduleEntry, Scheduler, BarrierOp, BarrierKind,
};
use crate::shader::{PipelineLayoutDescr, ShaderId, ShaderSourceSet, SpecConstantSet};

use crate::parameter::{Parameter, ParameterStore};

use crate::graph_borrowable::{DynGraphBorrow, GraphBorrow, GraphBorrowable};

mod macros;

pub mod unsafe_bump;
use unsafe_bump::Bump;

mod context;
pub use context::{GraphConstructCtx, PassConstructCtx, StandaloneConstructCtx};

pub struct GfxSchedulerTypes<B: hal::Backend>(PhantomData<B>);
impl<B: hal::Backend> crate::scheduler::SchedulerTypes for GfxSchedulerTypes<B> {
    type Image = GraphImage<B>;
    type Buffer = B::Buffer;
    type Semaphore = B::Semaphore;

    // TODO blocked by:
    // https://github.com/rust-lang/rust/issues/78459 :(
    //type NodeValue = Callback<B, Bump>;
    type NodeValue = Callback<B, std::alloc::Global>;
}

pub enum GraphImage<B: hal::Backend> {
    Image(B::Image),
    SwapchainImage(<B::Surface as PresentationSurface<B>>::SwapchainImage),
}
impl<B: hal::Backend> GraphImage<B> {
    fn image(&self) -> &B::Image {
        match self {
            GraphImage::Image(image) => image,
            GraphImage::SwapchainImage(swapchain_image) => swapchain_image.borrow(),
        }
    }
}

pub enum Callback<B: hal::Backend, A: Allocator> {
    None,
    Standalone {
        node: GraphGenerationNodeId,
        callback:
            Box<dyn FnOnce(&mut dyn Any, &Factory<B>, &mut ExecCtx<B>, &mut Queue<B>) + 'static, A>,
    },
    Pass {
        node: GraphGenerationNodeId,
        callback: Box<dyn FnOnce(&mut dyn Any, &Factory<B>, &mut ExecCtx<B>) + 'static, A>,
    },
    Present {
        node: GraphGenerationNodeId,
        surface: GraphBorrow<rendy_wsi::Surface<B>>,
        image: ImageId,
        result_callback: Box<
            dyn FnOnce(
                &mut dyn Any,
                Result<Option<hal::window::Suboptimal>, hal::window::PresentError>,
            ),
        >,
        semaphore: SemaphoreId,
    },
}
impl<B: hal::Backend, A: Allocator> Default for Callback<B, A> {
    fn default() -> Self {
        Callback::None
    }
}
impl<B: hal::Backend, A: Allocator> Callback<B, A> {
    fn standalone(
        self,
    ) -> (
        GraphGenerationNodeId,
        Box<dyn FnOnce(&mut dyn Any, &Factory<B>, &mut ExecCtx<B>, &mut Queue<B>) + 'static, A>,
    ) {
        match self {
            Callback::Standalone { node, callback } => (node, callback),
            _ => unreachable!(),
        }
    }
    fn pass(
        self,
    ) -> (
        GraphGenerationNodeId,
        Box<dyn FnOnce(&mut dyn Any, &Factory<B>, &mut ExecCtx<B>) + 'static, A>,
    ) {
        match self {
            Callback::Pass { node, callback } => (node, callback),
            _ => unreachable!(),
        }
    }
}

pub trait Node<B: hal::Backend>: 'static {
    type Argument;
    type Result;

    fn construct(
        &mut self,
        factory: &Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        arg: Self::Argument,
    ) -> Result<Self::Result, ()>;
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct GraphGenerationNodeId(u32);
entity_impl!(GraphGenerationNodeId);

pub struct Graph<'a, B: hal::Backend> {
    factory: &'a Factory<B>,

    procedural: ProceduralBuilder<GfxSchedulerTypes<B>>,
    nodes: PrimaryMap<GraphGenerationNodeId, DynGraphBorrow>,

    semaphores: HashMap<SemaphoreId, B::Semaphore>,

    cache: Arc<Cache<B>>,

    /// A bump allocator for the current Graph generation.
    /// At the end of a generation, this will be cleared,
    /// meaning we NEED to make sure everything allocated in here
    /// is dropped before then.
    ///
    /// TODO: Usage is blocked by
    /// https://github.com/rust-lang/rust/issues/78459 :(
    generation_alloc: Bump,
}

pub enum Resource<B: hal::Backend> {
    Image(GraphImage<B>),
    Buffer(B::Buffer),
}
impl<B: hal::Backend> Resource<B> {
    fn image_ref(&self) -> &GraphImage<B> {
        match self {
            Resource::Image(image) => image,
            _ => panic!(),
        }
    }
}

impl<'a, B: hal::Backend> Graph<'a, B> {
    pub fn new(factory: &'a Factory<B>) -> Self {
        Self {
            factory,
            procedural: ProceduralBuilder::new(),
            nodes: PrimaryMap::new(),
            semaphores: HashMap::new(),
            cache: Arc::new(Cache::new(factory)),
            generation_alloc: Bump::new(),
        }
    }

    pub fn cache(&self) -> &Arc<Cache<B>> {
        &self.cache
    }

    pub fn construct_node<N: Node<B>>(
        &mut self,
        node: &mut GraphBorrowable<N>,
        argument: N::Argument,
    ) -> N::Result {
        let factory = &*self.factory;

        let mut node = node.take_borrow();

        let mut ctx = GraphConstructCtx {
            node_id: self.nodes.next_key(),
            inner: self,
        };
        let result = node.construct(factory, &mut ctx, argument).unwrap();

        let node_id = self.nodes.push(node.into_any());

        result
    }

    fn commit_standalone<F>(&mut self, node: GraphGenerationNodeId, exec: F)
    where
        F: FnOnce(&mut dyn Any, &Factory<B>, &mut ExecCtx<B>, &mut Queue<B>) + 'static,
    {
        self.procedural.commit(Callback::Standalone {
            node,
            callback: Box::new(exec),
        });
    }

    fn commit_pass<F>(&mut self, node: GraphGenerationNodeId, exec: F)
    where
        F: FnOnce(&mut dyn Any, &Factory<B>, &mut ExecCtx<B>) + 'static,
    {
        self.procedural.commit(Callback::Pass {
            node,
            callback: Box::new(exec),
        });
    }

    fn add_present<F>(
        &mut self,
        surface: GraphBorrow<rendy_wsi::Surface<B>>,
        node_id: GraphGenerationNodeId,
        image: ImageId,
        result_callback: F,
    ) where
        F: FnOnce(&mut dyn Any, Result<Option<hal::window::Suboptimal>, hal::window::PresentError>)
            + 'static,
    {
        let sync_point = self.procedural.sync_point_get(image);
        let semaphore_id = self.procedural.sync_point_to_semaphore(sync_point);

        self.procedural.start_standalone();
        self.procedural
            .use_image(
                image,
                ImageUsage {
                    layout: hal::image::Layout::Present,
                    stages: hal::pso::PipelineStage::BOTTOM_OF_PIPE,
                    access: hal::image::Access::empty(),
                },
            )
            .unwrap();
        self.procedural.commit(Callback::Present {
            node: node_id,
            surface,
            image,
            result_callback: Box::new(result_callback),
            semaphore: semaphore_id,
        });

        self.procedural.mark_dead(image);
    }

    pub fn make_shader_set(
        &mut self,
        source: ShaderSourceSet,
        spec_constants: SpecConstantSet,
    ) -> ShaderId {
        let key = Arc::new(ShaderSetKey {
            source,
            spec_constants,
        });

        let reflection = key.source.reflect().unwrap();
        self.cache.make_shader_set(self.factory, key, reflection)
    }

    pub fn schedule(&mut self, frames: &mut Frames<B>, queue: &mut Queue<B>) {
        use hal::pool::CommandPool;

        self.procedural.postprocess();

        let mut scheduler = Scheduler::new();
        scheduler.plan(&self.procedural);

        let mut resources: BTreeMap<ResourceId, (Resource<B>, Option<B::Semaphore>)> =
            BTreeMap::new();

        for resource in scheduler.iter_live_resources() {
            let data = self.procedural.get_resource_info_mut(resource);
            match &mut data.kind {
                ResourceKind::Image(image_data) => {
                    let (image, acquire) = match image_data.source_mut() {
                        ImageSource::Owned => todo!(),
                        ImageSource::Provided { image, acquire, .. } => {
                            (image.take().unwrap(), acquire.take())
                        }
                    };

                    println!("{:?}", image_data.info());

                    resources.insert(resource, (Resource::Image(image), acquire));
                }
                ResourceKind::Buffer(buffer_data) => {
                    todo!()
                }
                ResourceKind::Alias(_) => unreachable!(),
            }
        }

        let mut schedule_iterator = ScheduleIterator::new();

        let mut pool = frames.get_command_pool(self.factory);
        let mut command_buffers = Vec::new();

        while let Some(current) = schedule_iterator.next(&scheduler) {
            let mut command_buffer = unsafe { pool.allocate_one(hal::command::Level::Primary) };
            unsafe {
                command_buffer.begin(
                    hal::command::CommandBufferFlags::ONE_TIME_SUBMIT,
                    hal::command::CommandBufferInheritanceInfo::default(),
                );
            }

            let mut exec_ctx = crate::exec::ExecCtx {
                phantom: PhantomData,

                factory: self.factory,
                cache: self.cache.clone(),

                active_subpass: None,

                command_buffer,
            };

            self.record_barriers(
                &scheduler,
                &resources,
                schedule_iterator.idx(),
                &mut exec_ctx.command_buffer,
            );

            match current {
                Current::Pass(pass) => {
                    let render_pass_id = self.begin_render_pass(
                        &scheduler,
                        &resources,
                        pass.pass,
                        &mut exec_ctx.command_buffer,
                    );

                    for (subpass_idx, (schedule_idx, advance)) in pass.schedule_entries.enumerate()
                    {
                        let entity_id = scheduler.scheduled_order[schedule_idx].entity_id();
                        let callback_enum = self.procedural.remove_data(entity_id).unwrap();
                        let (node_id, callback) = callback_enum.pass();

                        exec_ctx.active_subpass = Some(SubpassData {
                            render_pass: render_pass_id,
                            subpass_idx: subpass_idx.try_into().unwrap(),
                        });
                        callback(&mut *self.nodes[node_id], &*self.factory, &mut exec_ctx);

                        if advance {
                            // Continuing with the same render pass, call
                            // next_subpass on command buffer.
                            unsafe {
                                exec_ctx
                                    .command_buffer
                                    .next_subpass(hal::command::SubpassContents::Inline);
                            }
                        } else {
                            // End of render pass.
                            unsafe {
                                exec_ctx.command_buffer.end_render_pass();
                            }
                        }
                    }
                }
                Current::General(general) => {
                    let entity_id = general.entity;
                    let callback_enum = self.procedural.remove_data(entity_id).unwrap();

                    match callback_enum {
                        Callback::Present {
                            node,
                            mut surface,
                            image,
                            result_callback,
                            semaphore,
                        } => {
                            let image = match resources.remove(&image.into()).unwrap() {
                                (Resource::Image(GraphImage::SwapchainImage(sc_image)), _) => {
                                    sc_image
                                }
                                _ => panic!(),
                            };

                            let result = unsafe {
                                queue.raw().present(
                                    surface.raw_mut(),
                                    image,
                                    self.semaphores.get_mut(&semaphore),
                                )
                            };

                            result_callback(&mut *self.nodes[node], result);
                        }
                        _ => panic!(),
                    }
                }
            }

            let next_idx = schedule_iterator.next_idx();
            let sync_slot = &scheduler.sync_strategy.slots[next_idx];

            unsafe {
                exec_ctx.command_buffer.finish();

                for signal in sync_slot.external_signal.iter() {
                    match signal {
                        ExternalSignal::Semaphore(semaphore_id) => {
                            let semaphore = frames.get_semaphore(self.factory);
                            self.semaphores.insert(*semaphore_id, semaphore);
                        }
                        ExternalSignal::Fence(fence_id) => todo!(),
                    }
                }

                let signals = sync_slot
                    .external_signal
                    .iter()
                    .filter_map(|sig| match sig {
                        ExternalSignal::Semaphore(semaphore_id) => Some(&self.semaphores[semaphore_id]),
                        _ => None,
                    });

                queue.raw().submit(
                    std::iter::once(&exec_ctx.command_buffer),
                    std::iter::empty(),
                    signals,
                    None,
                );
            }

            command_buffers.push(exec_ctx.command_buffer);
        }

        let mut end_fence = frames.get_fence(self.factory);
        unsafe {
            queue.raw().submit(
                std::iter::empty(),
                std::iter::empty(),
                std::iter::empty(),
                Some(&mut end_fence),
            );
        }

        {
            let frame = frames.current();
            frame.semaphores.extend(self.semaphores.drain().map(|(_k, v)| v));
            frame.command_pools.push((pool, command_buffers));
        }
        frames.wait_fence(end_fence);

        self.reset_generation();

        //queue.raw().wait_idle().unwrap();
        //std::thread::sleep(std::time::Duration::new(10, 0));
    }

    fn record_barriers(
        &self,
        scheduler: &Scheduler,
        resources: &BTreeMap<ResourceId, (Resource<B>, Option<B::Semaphore>)>,
        sync_idx: usize,
        buffer: &mut B::CommandBuffer,
    ) {
        let sync_slot = &scheduler.sync_strategy.slots[sync_idx];
        let barriers = &scheduler.sync_strategy.barriers[sync_slot.barrier_range.clone().unwrap()];

        for barrier in barriers {
            let hal_barrier = match barrier.op {
                BarrierOp::Barrier => {
                    match barrier.kind.clone() {
                        BarrierKind::Image {
                            states, target, range, families,
                        } => {
                            let target = &resources[&target].0;

                            hal::memory::Barrier::Image {
                                states,
                                target: target.image_ref().image(),
                                range,
                                families,
                            }
                        },
                        _ => todo!(),
                    }
                },
                _ => todo!(),
            };

            unsafe {
                buffer.pipeline_barrier(
                    barrier.stages.clone(),
                    hal::memory::Dependencies::empty(),
                    std::iter::once(hal_barrier),
                );
            }
        }
    }

    fn begin_render_pass(
        &self,
        scheduler: &Scheduler,
        resources: &BTreeMap<ResourceId, (Resource<B>, Option<B::Semaphore>)>,
        render_pass: RenderPass,
        command_buffer: &mut B::CommandBuffer,
    ) -> RenderPassId {
        let render_pass_id = self.make_render_pass(&scheduler, render_pass);
        let pass_data = &scheduler.passes[render_pass];

        let extent = pass_data.extent.unwrap();

        let attachments_descr = pass_data.attachment_data.iter().map(|a| {
            let resource = a.resource;
            let info = self.procedural.get_image_info(resource.into());

            hal::image::FramebufferAttachment {
                usage: hal::image::Usage::all(),
                view_caps: hal::image::ViewCapabilities::empty(),
                format: info.format,
            }
        });

        let render_pass = self.cache.get_render_pass(render_pass_id);
        let framebuffer = unsafe {
            self.factory
                .device()
                .create_framebuffer(&*render_pass, attachments_descr, extent)
                .unwrap()
        };

        let attachments = pass_data.attachment_data.iter().map(|a| {
            let (item, acquire) = &resources[&a.resource];

            let image_view: &B::ImageView = match item {
                Resource::Image(GraphImage::SwapchainImage(swapchain_image)) => {
                    swapchain_image.borrow()
                }
                Resource::Image(GraphImage::Image(_)) => todo!(),
                _ => unreachable!(),
            };

            hal::command::RenderAttachmentInfo {
                image_view,
                clear_value: hal::command::ClearValue::default(),
            }
        });

        unsafe {
            command_buffer.begin_render_pass(
                &*render_pass,
                &framebuffer,
                extent.rect(),
                attachments,
                hal::command::SubpassContents::Inline,
            );
        }

        render_pass_id
    }

    fn make_render_pass(&self, scheduler: &Scheduler, render_pass: RenderPass) -> RenderPassId {
        use crate::scheduler::{
            input::{ResourceId, SchedulerInput},
            resources::ImageMode,
        };

        let pass_data = &scheduler.passes[render_pass];

        let mut refs: BTreeMap<ResourceId, u32> = BTreeMap::new();
        let mut attachments = Vec::new();
        let mut attachment_map = BTreeMap::new();
        for (idx, res) in pass_data.attachment_data.iter().enumerate() {
            let resource = &self.procedural.resources[res.resource];
            let kind = resource.kind.image_ref().unwrap().info();

            attachment_map.insert(res.resource, (idx, false));

            let attachment = hal::pass::Attachment {
                format: Some(kind.format),
                samples: 1, // TODO
                layouts: res.layouts.clone(),
                // TODO distinguish depth vs stencil for depth stencil buffer
                ops: res.ops,
                stencil_ops: res.stencil_ops,
            };

            attachments.push(attachment);
            refs.insert(res.resource, idx as u32);
        }

        scheduler.debug_print_schedule_matrix();

        let subpasses: Vec<_> = pass_data
            .entities
            .as_slice(&scheduler.entity_list_pool)
            .iter()
            .map(|entity| {
                let attachments = &self.procedural.get_attachments(*entity).unwrap();

                let mut desc_key = SubpassDescKey {
                    colors: Vec::new(),
                    depth_stencil: None,
                    inputs: Vec::new(),
                    resolves: Vec::new(),
                    preserves: Vec::new(),
                };

                if let Some(image_id) = attachments.depth {
                    let usage_kind = scheduler
                        .usage_kind(*entity, image_id.into())
                        .unwrap()
                        .attachment_layout()
                        .unwrap();

                    let (idx, prev_used) = attachment_map.get_mut(&image_id.into()).unwrap();
                    assert!(!*prev_used);
                    *prev_used = true;

                    desc_key.depth_stencil = Some((*idx, usage_kind));
                }

                for image_id in attachments.color.iter().cloned() {
                    let usage_kind = scheduler
                        .usage_kind(*entity, image_id.into())
                        .unwrap()
                        .attachment_layout()
                        .unwrap();

                    let (idx, prev_used) = attachment_map.get_mut(&image_id.into()).unwrap();
                    assert!(!*prev_used);
                    *prev_used = true;

                    desc_key.colors.push((*idx, usage_kind));
                }

                for image_id in attachments.input.iter().cloned() {
                    let usage_kind = scheduler
                        .usage_kind(*entity, image_id.into())
                        .unwrap()
                        .attachment_layout()
                        .unwrap();

                    let (idx, prev_used) = attachment_map.get_mut(&image_id.into()).unwrap();
                    assert!(!*prev_used);
                    *prev_used = true;

                    desc_key.inputs.push((*idx, usage_kind));
                }

                desc_key
                    .preserves
                    .extend(attachment_map.values().filter_map(|(idx, prev_used)| {
                        if !prev_used {
                            Some(*idx)
                        } else {
                            None
                        }
                    }));

                for (_idx, prev_used) in attachment_map.values_mut() {
                    *prev_used = false;
                }

                desc_key
            })
            .collect();

        let mut dependencies = Vec::new();

        let key = Arc::new(RenderPassKey {
            attachments,
            subpasses,
            dependencies,
        });

        self.cache.make_render_pass(self.factory, key)
    }

    fn reset_generation(&mut self) {
        self.procedural.clear();
        self.nodes.clear();

        // This MUST be called AFTER all allocated values are dropped.
        unsafe {
            self.generation_alloc.reset();
        }
    }
}
