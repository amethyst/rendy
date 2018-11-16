// use std::{
//     borrow::Borrow, collections::HashMap, iter::once, marker::PhantomData, ops::AddAssign,
//     sync::{atomic::AtomicUsize, Arc},
// };

use chain;
use factory::Factory;
use frame::{Frame, Frames};
use memory::MemoryUsageValue;
use resource::{buffer, image};
use wsi::Target;

use crate::{
    ImageOrTarget,
    node::{
        AnyNode, NodeBuilder,
    },
};

#[derive(Debug)]
struct Presentable<B: gfx_hal::Backend> {
    target: Target<B>,
    signal: usize,
    wait: Vec<usize>,
    owner: gfx_hal::queue::QueueFamilyId,
    next_index: u32,
}

/// Graph that renders whole frame.
#[derive(Debug)]
pub struct Graph<B: gfx_hal::Backend, T: ?Sized> {
    nodes: Vec<Box<dyn AnyNode<B, T>>>,
    schedule: chain::Schedule<chain::SyncData<usize, usize>>,
    semaphores: Vec<B::Semaphore>,
    buffers: Vec<buffer::Buffer<B>>,
    images: Vec<image::Image<B>>,
    presentables: Vec<Presentable<B>>,
    frames: Frames<B>,
    inflight: u64,
}

impl<B, T> Graph<B, T>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    /// Perform graph execution.
    /// Run every node of the graph and submit resulting command buffers to the queues.
    ///
    /// # Parameters
    ///
    /// `command_queues`   - function to get `CommandQueue` by `QueueFamilyId` and index.
    ///               `Graph` guarantees that it will submit only command buffers
    ///               allocated from the command pool associated with specified `QueueFamilyId`.
    ///
    /// `device`    - `Device<B>` implementation. `B::Device` or wrapper.
    ///
    /// `aux`       - auxiliary data that `Node`s use.
    ///
    /// `fences`    - vector of fences that will be signaled after all commands are complete.
    ///               Fences that are attached to last submissions of every queue are reset.
    ///               This function may not use all fences. Unused fences are left in signalled state.
    ///               If this function needs more fences they will be allocated from `device` and pushed to this `Vec`.
    ///               So it's OK to start with empty `Vec`.
    pub fn run<'a, Q: 'a>(
        &mut self,
        factory: &mut Factory<B>,
        aux: &mut T,
    ) {
        if self.frames.next().index() >= self.inflight {
            let wait = self.frames.next().index() - self.inflight;
            self.frames.wait_complete(wait, factory);
        }

        let empty_submit_info = || SubmitInfo::<B> {
            buffers: Default::default(),
            waits: Default::default(),
            signals: Default::default(),
        };

        for presentable in &mut self.presentables {
            presentable.next_index = unsafe {
                gfx_hal::Swapchain::acquire_image(
                    presentable.target.swapchain_mut(),
                    !0,
                    gfx_hal::FrameSync::Semaphore(&self.semaphores[presentable.signal]),
                )
            }.expect("Swapchain errors are not handled yet");
        }

        let mut ready = empty_submit_info();

        for submission in self.schedule.ordered() {
            let sid = submission.id();
            let qid = sid.queue();

            if let Some(node) = self.nodes.get_mut(submission.node()) {
                let submit = node.run(
                    factory,
                    aux,
                    &self.frames,
                );

                let family = factory.family_mut(gfx_hal::queue::QueueFamilyId(qid.family().0));
                let ref mut queue = family.queues_mut()[qid.index()];

                let mut fence_index = 0;
                let last_in_queue = sid.index() + 1 == self.schedule.queue(qid).unwrap().len();
                let fence = if last_in_queue {
                    fence_index += 1;
                    Some(&self.frames.next().fences()[fence_index - 1])
                } else {
                    None
                };

                let ref semaphores = self.semaphores;

                ready.waits.extend(submission.sync().wait.iter().map(|wait|(&semaphores[*wait.semaphore()], wait.stage())));
                ready.buffers.push(submit.into_raw());
                ready.signals.extend(submission.sync().signal.iter().map(|signal| &semaphores[*signal.semaphore()]));

                unsafe {
                    gfx_hal::queue::RawCommandQueue::submit_raw(
                        queue,
                        gfx_hal::queue::RawSubmission {
                            cmd_buffers: &ready.buffers,
                            wait_semaphores: &ready.waits,
                            signal_semaphores: &ready.signals,
                        },
                        fence,
                    );
                    ready = empty_submit_info();
                }
            }
        }

        for presentable in &self.presentables {
            let family = factory.family_mut(presentable.owner);
            let ref mut queue = family.queues_mut()[0];

            gfx_hal::queue::RawCommandQueue::present(
                queue,
                Some((presentable.target.swapchain(), presentable.next_index)),
                presentable.wait.iter().map(|&index| &self.semaphores[index]),
            ).expect("Device lost is not handled yet");
        }
    }
}

#[derive(Debug)]
struct SubmitInfo<'a, B: gfx_hal::Backend> {
    buffers: smallvec::SmallVec<[B::CommandBuffer; 16]>,
    waits: smallvec::SmallVec<[(&'a B::Semaphore, gfx_hal::pso::PipelineStage); 16]>,
    signals: smallvec::SmallVec<[&'a B::Semaphore; 16]>,
}

#[derive(Clone, Copy, Debug)]
enum ImageOrTargetInfo {
    Image(image::Info, u64, MemoryUsageValue),
    Target(usize, gfx_hal::image::Usage),
}

/// Build graph from nodes and resource.
#[derive(Debug)]
pub struct GraphBuilder<B: gfx_hal::Backend, T: ?Sized> {
    nodes: Vec<NodeBuilder<B, T>>,
    buffers: Vec<(buffer::Info, u64, MemoryUsageValue)>,
    images_or_targets: Vec<(ImageOrTargetInfo, Option<gfx_hal::command::ClearValue>)>,
    target_count: usize,
}

/// Id of the buffer in graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferId(usize);

/// Id of the image (or target) in graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageId(usize);

/// Id of the node in graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(usize);

impl<B, T> GraphBuilder<B, T>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    /// Create new `GraphBuilder`
    pub fn new() -> Self {
        GraphBuilder {
            nodes: Vec::new(),
            buffers: Vec::new(),
            images_or_targets: Vec::new(),
            target_count: 0,
        }
    }

    /// Create new buffer owned by graph.
    pub fn create_buffer(&mut self,
        size: u64,
        usage: gfx_hal::buffer::Usage,
        align: u64,
        memory: MemoryUsageValue,
    ) -> BufferId {
        self.buffers.push((
            buffer::Info {
                size,
                usage,
            }, 
            align,
            memory,
        ));
        BufferId(self.buffers.len() - 1)
    }

    /// Create new image owned by graph.
    pub fn create_image(
        &mut self,
        kind: gfx_hal::image::Kind,
        levels: gfx_hal::image::Level,
        format: gfx_hal::format::Format,
        tiling: gfx_hal::image::Tiling,
        view_caps: gfx_hal::image::ViewCapabilities,
        usage: gfx_hal::image::Usage,
        align: u64,
        memory: MemoryUsageValue,
        clear: Option<gfx_hal::command::ClearValue>,
    ) -> ImageId {
        self.images_or_targets.push((ImageOrTargetInfo::Image(image::Info {
            kind,
            levels,
            format,
            tiling,
            view_caps,
            usage,
        }, align, memory), clear));
        ImageId(self.images_or_targets.len() - 1)
    }

    /// Create new image owned by graph.
    pub fn add_target(
        &mut self,
        usage: gfx_hal::image::Usage,
        clear: Option<gfx_hal::command::ClearValue>,
    ) -> ImageId {
        self.images_or_targets.push((ImageOrTargetInfo::Target(self.target_count, usage), clear));
        self.target_count += 1;
        ImageId(self.images_or_targets.len() - 1)
    }

    /// Add node to the graph.
    pub fn add_node(&mut self, builder: NodeBuilder<B, T>) -> NodeId {
        self.nodes.push(builder);
        NodeId(self.nodes.len() - 1)
    }

    /// Build `Graph`.
    ///
    /// # Parameters
    ///
    /// `frames`        - maximum number of frames `Graph` will render simultaneously.
    ///
    /// `families`      - `Iterator` of `B::QueueFamily`s.
    ///
    /// `device`    - `Device<B>` implementation. `B::Device` or wrapper.
    ///
    /// `aux`       - auxiliary data that `Node`s use.
    pub fn build(
        self,
        factory: &mut Factory<B>,
        aux: &mut T,
        targets: impl IntoIterator<Item = winit::Window>,
    ) -> Result<Graph<B, T>, failure::Error> {
        log::trace!("Schedule nodes execution");
        let chain_nodes: Vec<chain::Node> = self.nodes
            .iter()
            .enumerate()
            .map(|(i, b)| b.chain(i, &factory))
            .collect();

        let mut chains = chain::collect(chain_nodes, |qid| {
            factory.family(qid).queues().len()
        });

        log::trace!("Scheduled nodes execution {:#?}", chains);

        log::trace!("Allocate buffers");
        let buffers: Vec<buffer::Buffer<B>> = self
            .buffers
            .iter()
            .enumerate()
            .map(|(index, &(ref info, align, memory))| {
                let usage = chains
                    .buffers
                    .get(&chain::Id(index))
                    .map_or(gfx_hal::buffer::Usage::empty(), |chain| chain.usage());

                factory.create_buffer(
                    align,
                    info.size,
                    (info.usage | usage, memory)
                )
            })
            .collect::<Result<_, _>>()?;

        let mut targets: Vec<_> = targets.into_iter().map(Some).collect();
        let mut inflight = 3;

        let mut presenting_sids = Vec::new();

        log::trace!("Allocate images and create targets");
        let images_or_targets = self
            .images_or_targets
            .iter()
            .enumerate()
            .map(|(index, (image, clear))| {
                match image {
                    &ImageOrTargetInfo::Image(ref info, align, memory) => {
                        let usage = chains
                            .images
                            .get(&chain::Id(index + buffers.len()))
                            .map_or(gfx_hal::image::Usage::empty(), |chain| chain.usage());

                        factory.create_image(
                            align,
                            info.kind,
                            info.levels,
                            info.format,
                            info.tiling,
                            info.view_caps,
                            (info.usage | usage, memory),
                        ).map(ImageOrTarget::Image)
                    },

                    &ImageOrTargetInfo::Target(target_index, usage) => {
                        // Add preseting quasi-submission.
                        let image_chain_id = chain::Id(index + buffers.len());
                        let chain = chains.images.get_mut(&image_chain_id).unwrap();

                        let owner = chain.links().last().unwrap().family();
                        let family = chains.schedule.family_mut(owner).unwrap();
                        let imaginary_queue = family.queue_count();
                        let queue = family.ensure_queue(chain::QueueId::new(owner, imaginary_queue));
                        assert!(queue.iter().all(|s| s.node() == self.nodes.len()), "Only presenting quasi-submissions in imaginary queue");
                        let sid = queue.add_submission(
                            self.nodes.len(),
                            !0,
                            !0,
                            chain::Unsynchronized,
                        );

                        chain.add_link(chain::Link::new(chain::LinkNode {
                            sid,
                            state: chain::State {
                                access: gfx_hal::image::Access::empty(),
                                layout: gfx_hal::image::Layout::Present,
                                stages: gfx_hal::pso::PipelineStage::TOP_OF_PIPE,
                                usage: gfx_hal::image::Usage::empty(),
                            }
                        }));

                        let submission = queue.submission_mut(sid).unwrap();
                        submission.set_link(image_chain_id, chain.links().len() - 1);

                        presenting_sids.push(sid);

                        factory.create_target(targets[target_index].take().unwrap(), 3, usage)
                            .map(|target| {
                                inflight = std::cmp::min(inflight, target.images().len());
                                ImageOrTarget::Target(target)
                            })
                    }
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        log::trace!("Synchronize");
        let mut semaphores = 0..;
        let schedule = chain::sync(&chains, || {
            let id = semaphores.next().unwrap();
            (id, id)
        });

        log::trace!("Schedule: {:#?}", schedule);
        let mut queues = 0;

        log::trace!("Build nodes");
        let mut built_nodes: Vec<_> = (0 .. self.nodes.len()).map(|_| None).collect();
        for family in schedule.iter() {
            log::trace!("For family {:#?}", family);
            for queue in family.iter() {
                queues += 1;
                log::trace!("For queue {:#?}", queue.id());
                for submission in queue.iter() {
                    log::trace!("For submission {:#?}", submission.id());
                    let ref builder = self.nodes[submission.node()];
                    log::trace!("Build node {:#?}", builder);
                    let node = builder.build(
                        factory,
                        aux,
                        &buffers,
                        &images_or_targets,
                    );
                    built_nodes[submission.node()] = Some(node);
                }
            }
        }

        log::trace!("Create {} semaphores", semaphores.start);
        let semaphores = (0..semaphores.start)
                .map(|_| factory.create_semaphore())
                .collect::<Result<_, _>>()?;

        let mut images = Vec::new();
        let mut presentables = Vec::new();

        let mut presenting_sids = presenting_sids.into_iter();

        images_or_targets.into_iter().enumerate().for_each(|(index, iot)| match iot {
            ImageOrTarget::Image(image) => images.push(image),
            ImageOrTarget::Target(target) => {
                let sid = presenting_sids.next().unwrap();
                let sync_data = schedule.submission(sid).unwrap().sync();

                assert!(sync_data.acquire.buffers.is_empty(), "Presentation can't insert barriers");
                assert!(sync_data.acquire.images.is_empty(), "Presentation can't insert barriers");
                assert!(sync_data.release.buffers.is_empty(), "Presentation can't insert barriers");
                assert!(sync_data.release.images.is_empty(), "Presentation can't insert barriers");

                let wait = sync_data.wait.iter().map(|wait| {
                    assert_eq!(wait.stage(), gfx_hal::pso::PipelineStage::TOP_OF_PIPE);
                    *wait.semaphore()
                }).collect();

                assert_eq!(sync_data.signal.len(), 1, "Presentation can't signal more than 1 semaphore.");

                presentables.push(Presentable {
                    target,
                    wait,
                    signal: *sync_data.signal[0].semaphore(),
                    owner: sid.family(),
                    next_index: 0,
                })
            }
        });

        Ok(Graph {
            nodes: built_nodes.into_iter().map(|node| node.unwrap()).collect(),
            schedule,
            semaphores,
            buffers,
            images,
            presentables,
            inflight: inflight as u64,
            frames: Frames::new(factory, queues),
        })
    }
}
