use {
    crate::{
        chain, command,
        factory::Factory,
        frame::{Fences, Frames},
        memory::MemoryUsageValue,
        node::{DynNode, NodeBuilder},
        resource::{buffer, image},
        BufferId, ImageId, NodeId,
    },
    gfx_hal::Backend,
};

// TODO: Use actual limits.
const UNIVERSAL_ALIGNMENT: u64 = 512;

#[derive(Debug)]
struct GraphNode<B: Backend, T: ?Sized> {
    node: Box<dyn DynNode<B, T>>,
    queue: command::QueueId,
}

/// Graph that renders whole frame.
#[derive(Debug)]
pub struct Graph<B: Backend, T: ?Sized> {
    nodes: Vec<GraphNode<B, T>>,
    schedule: chain::Schedule<chain::SyncData<usize, usize>>,
    semaphores: Vec<B::Semaphore>,
    buffers: Vec<buffer::Buffer<B>>,
    images: Vec<image::Image<B>>,
    frames: Frames<B>,
    fences: Vec<Fences<B>>,
    inflight: u64,
}

impl<B, T> Graph<B, T>
where
    B: Backend,
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
    ///               This function may not use all fences. Unused fences are left in signaled state.
    ///               If this function needs more fences they will be allocated from `device` and pushed to this `Vec`.
    ///               So it's OK to start with empty `Vec`.
    pub fn run(&mut self, factory: &mut Factory<B>, aux: &mut T) {
        if self.frames.next().index() >= self.inflight {
            let wait = self.frames.next().index() - self.inflight;
            let ref mut self_fences = self.fences;
            self.frames.wait_complete(wait, factory, |mut fences| {
                factory.reset_fences(&mut fences).unwrap();
                self_fences.push(fences);
            });
        }

        let mut fences = self.fences.pop().unwrap_or_else(Fences::<B>::default);
        let mut fences_used = 0;
        let ref semaphores = self.semaphores;

        for submission in self.schedule.ordered() {
            log::trace!("Run node {}", submission.node());
            let sid = submission.id();
            let qid = sid.queue();

            let GraphNode { node, queue } = self
                .nodes
                .get_mut(submission.node())
                .expect("Submission references node with out of bound index");
            debug_assert_eq!(
                (qid.family(), qid.index()),
                (queue.family(), queue.index()),
                "Node's queue doesn't match schedule"
            );

            let last_in_queue = sid.index() + 1 == self.schedule.queue(qid).unwrap().len();
            let fence = if last_in_queue {
                if fences_used >= fences.len() {
                    fences.push(factory.create_fence(false).unwrap());
                }
                fences_used += 1;
                Some(&mut fences[fences_used - 1])
            } else {
                None
            };

            unsafe {
                node.run(
                    factory,
                    aux,
                    &self.frames,
                    *queue,
                    &submission
                        .sync()
                        .wait
                        .iter()
                        .map(|wait| {
                            log::trace!(
                                "Node {} waits for {}",
                                submission.node(),
                                *wait.semaphore()
                            );
                            (&semaphores[*wait.semaphore()], wait.stage())
                        })
                        .collect::<smallvec::SmallVec<[_; 16]>>(),
                    &submission
                        .sync()
                        .signal
                        .iter()
                        .map(|signal| {
                            log::trace!(
                                "Node {} signals {}",
                                submission.node(),
                                *signal.semaphore()
                            );
                            &semaphores[*signal.semaphore()]
                        })
                        .collect::<smallvec::SmallVec<[_; 16]>>(),
                    fence,
                )
            }
        }

        fences.truncate(fences_used);
        unsafe {
            self.frames.advance(fences);
        }
    }

    /// Get queue that will exeute given node.
    pub fn node_queue(&self, node: NodeId) -> command::QueueId {
        self.nodes[node.0].queue
    }

    /// Dispose of the `Graph`.
    pub fn dispose(self, factory: &mut Factory<B>, data: &mut T) {
        assert!(factory.wait_idle().is_ok());
        self.frames.dispose(factory);

        unsafe {
            // Device is idle.
            for node in self.nodes {
                node.node.dispose(factory, data);
            }

            for semaphore in self.semaphores {
                factory.destroy_semaphore(semaphore);
            }
        }
    }
}

/// Build graph from nodes and resource.
#[derive(Debug)]
pub struct GraphBuilder<B: Backend, T: ?Sized> {
    nodes: Vec<Box<dyn NodeBuilder<B, T>>>,
    buffers: Vec<(buffer::Info, MemoryUsageValue)>,
    images: Vec<(
        image::Info,
        MemoryUsageValue,
        Option<gfx_hal::command::ClearValue>,
    )>,
    target_count: usize,
}

impl<B, T> GraphBuilder<B, T>
where
    B: Backend,
    T: ?Sized,
{
    /// Create new `GraphBuilder`
    pub fn new() -> Self {
        GraphBuilder {
            nodes: Vec::new(),
            buffers: Vec::new(),
            images: Vec::new(),
            target_count: 0,
        }
    }

    /// Create new buffer owned by graph.
    pub fn create_buffer(&mut self, size: u64, memory: MemoryUsageValue) -> BufferId {
        self.buffers.push((
            buffer::Info {
                size,
                usage: gfx_hal::buffer::Usage::empty(),
            },
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
        memory: MemoryUsageValue,
        clear: Option<gfx_hal::command::ClearValue>,
    ) -> ImageId {
        self.images.push((
            image::Info {
                kind,
                levels,
                format,
                tiling: gfx_hal::image::Tiling::Optimal,
                view_caps: gfx_hal::image::ViewCapabilities::empty(),
                usage: gfx_hal::image::Usage::empty(),
            },
            memory,
            clear,
        ));
        ImageId(self.images.len() - 1)
    }

    /// Add node to the graph.
    pub fn add_node<N: NodeBuilder<B, T> + 'static>(&mut self, builder: N) -> NodeId {
        self.nodes.push(Box::new(builder));
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
    ) -> Result<Graph<B, T>, failure::Error> {
        log::trace!("Schedule nodes execution");
        let chain_nodes: Vec<chain::Node> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, b)| make_chain_node(&**b, i, &factory))
            .collect();

        let chains = chain::collect(chain_nodes, |qid| factory.family(qid).queues().len());
        log::trace!("Scheduled nodes execution {:#?}", chains);

        log::trace!("Allocate buffers");
        let mut buffers: Vec<Option<buffer::Buffer<B>>> = self
            .buffers
            .iter()
            .enumerate()
            .map(|(index, &(ref info, memory))| {
                chains
                    .buffers
                    .get(&chain::Id(index))
                    .map(|buffer| {
                        factory
                            .create_buffer(UNIVERSAL_ALIGNMENT, info.size, (buffer.usage(), memory))
                            .map(|buffer| Some(buffer))
                    })
                    .unwrap_or(Ok(None))
            })
            .collect::<Result<_, _>>()?;

        log::trace!("Allocate images");
        let mut images: Vec<Option<(image::Image<B>, _)>> = self
            .images
            .iter()
            .enumerate()
            .map(|(index, (info, memory, clear))| {
                chains
                    .images
                    .get(&chain::Id(index))
                    .map(|image| {
                        factory
                            .create_image(
                                UNIVERSAL_ALIGNMENT,
                                info.kind,
                                info.levels,
                                info.format,
                                info.tiling,
                                info.view_caps,
                                (image.usage(), *memory),
                            )
                            .map(|image| Some((image, *clear)))
                    })
                    .unwrap_or(Ok(None))
            })
            .collect::<Result<_, _>>()?;

        log::trace!("Synchronize");
        let mut semaphores = 0..;
        let mut schedule = chain::sync(&chains, || {
            let id = semaphores.next().unwrap();
            (id, id)
        });
        schedule.build_order();
        log::info!("Schedule: {:#?}", schedule);

        log::trace!("Build nodes");
        let mut built_nodes: Vec<_> = (0..self.nodes.len()).map(|_| None).collect();
        let mut node_descs: Vec<_> = self.nodes.into_iter().map(Some).collect();
        for family in schedule.iter() {
            log::trace!("For family {:#?}", family);
            for queue in family.iter() {
                log::trace!("For queue {:#?}", queue.id());
                for submission in queue.iter() {
                    log::trace!("For submission {:#?}", submission.id());
                    let builder = node_descs[submission.node()].take().unwrap();
                    log::trace!("Build node {:#?}", builder);
                    let node = builder.build_impl(
                        factory,
                        aux,
                        family.id(),
                        &mut buffers,
                        &mut images,
                        &chains,
                        &submission,
                    )?;
                    log::debug!("Node built: {:#?}", node);
                    built_nodes[submission.node()] = Some((node, submission.id().queue()));
                }
            }
        }

        log::debug!("Create {} semaphores", semaphores.start);
        let semaphores = (0..semaphores.start)
            .map(|_| factory.create_semaphore())
            .collect::<Result<_, _>>()?;

        Ok(Graph {
            nodes: built_nodes
                .into_iter()
                .map(Option::unwrap)
                .map(|(node, qid)| GraphNode {
                    node,
                    queue: command::QueueId(qid.family(), qid.index()),
                })
                .collect(),
            schedule,
            semaphores,
            buffers: buffers.into_iter().filter_map(|x| x).collect(),
            images: images
                .into_iter()
                .filter_map(|x| x)
                .map(|(image, _)| image)
                .collect(),
            inflight: 3,
            frames: Frames::new(),
            fences: Vec::new(),
        })
    }
}

fn make_chain_node<B, T>(
    builder: &dyn NodeBuilder<B, T>,
    id: usize,
    factory: &Factory<B>,
) -> chain::Node
where
    B: Backend,
    T: ?Sized,
{
    let buffers = builder.buffers();
    let images = builder.images();
    chain::Node {
        id,
        family: builder.family(factory.families()).unwrap(),
        dependencies: builder.dependencies().into_iter().map(|id| id.0).collect(),
        buffers: buffers
            .into_iter()
            .map(|(id, access)| {
                (
                    chain::Id(id.0),
                    chain::BufferState {
                        access: access.access,
                        stages: access.stages,
                        layout: (),
                        usage: access.usage,
                    },
                )
            })
            .collect(),
        images: images
            .into_iter()
            .map(|(id, access)| {
                (
                    chain::Id(id.0),
                    chain::ImageState {
                        access: access.access,
                        stages: access.stages,
                        layout: access.layout,
                        usage: access.usage,
                    },
                )
            })
            .collect(),
    }
}
