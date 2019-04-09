use {
    crate::{
        chain,
        command::{Families, FamilyId, QueueId},
        factory::Factory,
        frame::{Fences, Frame, Frames},
        memory::Data,
        node::{BufferBarrier, DynNode, ImageBarrier, NodeBuffer, NodeBuilder, NodeImage},
        resource::{Buffer, BufferInfo, Handle, Image, ImageInfo},
        util::{device_owned, DeviceId},
        BufferId, ImageId, NodeId,
    },
    gfx_hal::{queue::QueueFamilyId, Backend},
};

#[derive(Debug)]
struct GraphNode<B: Backend, T: ?Sized> {
    node: Box<dyn DynNode<B, T>>,
    queue: (usize, usize),
}

/// Graph that renders whole frame.
#[derive(Debug)]
pub struct Graph<B: Backend, T: ?Sized> {
    device: DeviceId,
    nodes: Vec<GraphNode<B, T>>,
    schedule: chain::Schedule<chain::SyncData<usize, usize>>,
    semaphores: Vec<B::Semaphore>,
    frames: Frames<B>,
    fences: Vec<Fences<B>>,
    inflight: u32,
    ctx: GraphContext<B>,
}

device_owned!(Graph<B, T: ?Sized>);

/// Graphics context contains all transient resources managed by graph.
#[derive(Debug)]
pub struct GraphContext<B: Backend> {
    buffers: Vec<Option<Handle<Buffer<B>>>>,
    images: Vec<Option<(Handle<Image<B>>, Option<gfx_hal::command::ClearValue>)>>,
}

impl<B: Backend> GraphContext<B> {
    fn alloc<'a>(
        factory: &Factory<B>,
        chains: &chain::Chains,
        buffers: impl IntoIterator<Item = &'a BufferInfo>,
        images: impl IntoIterator<Item = &'a (ImageInfo, Option<gfx_hal::command::ClearValue>)>,
    ) -> Result<Self, failure::Error> {
        log::trace!("Allocate buffers");
        let buffers: Vec<Option<Handle<Buffer<B>>>> = buffers
            .into_iter()
            .enumerate()
            .map(|(index, info)| {
                chains
                    .buffers
                    .get(&chain::Id(index))
                    .map(|buffer| {
                        factory
                            .create_buffer(
                                BufferInfo {
                                    usage: buffer.usage(),
                                    ..info.clone()
                                },
                                Data,
                            )
                            .map(|buffer| Some(buffer.into()))
                    })
                    .unwrap_or(Ok(None))
            })
            .collect::<Result<_, _>>()?;

        log::trace!("Allocate images");
        let images: Vec<Option<(Handle<Image<B>>, _)>> = images
            .into_iter()
            .enumerate()
            .map(|(index, (info, clear))| {
                chains
                    .images
                    .get(&chain::Id(index))
                    .map(|image| {
                        factory
                            .create_image(
                                ImageInfo {
                                    usage: image.usage(),
                                    ..info.clone()
                                },
                                Data,
                            )
                            .map(|image| Some((image.into(), *clear)))
                    })
                    .unwrap_or(Ok(None))
            })
            .collect::<Result<_, _>>()?;

        Ok(Self { buffers, images })
    }

    /// Get reference to transient image by id.
    pub fn get_image(&self, id: ImageId) -> Option<&Handle<Image<B>>> {
        self.get_image_with_clear(id).map(|(i, _)| i)
    }

    /// Get reference to transient image and clear value by id.
    pub fn get_image_with_clear(
        &self,
        id: ImageId,
    ) -> Option<(&Handle<Image<B>>, Option<gfx_hal::command::ClearValue>)> {
        self.images
            .get(id.0)
            .and_then(|x| x.as_ref())
            .map(|&(ref x, ref y)| (&*x, *y))
    }

    /// Get reference to transient buffer by id.
    pub fn get_buffer(&self, id: BufferId) -> Option<&Handle<Buffer<B>>> {
        self.buffers.get(id.0).and_then(|x| x.as_ref()).map(|x| &*x)
    }
}

impl<B, T> Graph<B, T>
where
    B: Backend,
    T: ?Sized,
{
    /// Perform graph execution.
    /// Run every node of the graph and submit resulting command buffers to the queues.
    pub fn run(&mut self, factory: &mut Factory<B>, families: &mut Families<B>, aux: &T) {
        self.assert_device_owner(factory.device());

        if self.frames.next().index() >= self.inflight as _ {
            let wait = Frame::with_index(self.frames.next().index() - self.inflight as u64);
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
                (QueueFamilyId(queue.0), queue.1),
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
                    &self.ctx,
                    factory,
                    families.family_by_index_mut(queue.0).queue_mut(queue.1),
                    aux,
                    &self.frames,
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
        self.frames.advance(fences);
    }

    /// Get queue that will exeute given node.
    pub fn node_queue(&self, node: NodeId) -> QueueId {
        let (f, i) = self.nodes[node.0].queue;
        QueueId {
            family: FamilyId {
                device: self.device,
                index: f,
            },
            index: i,
        }
    }

    /// Dispose of the `Graph`.
    pub fn dispose(self, factory: &mut Factory<B>, data: &T) {
        self.assert_device_owner(factory.device());

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
    buffers: Vec<BufferInfo>,
    images: Vec<(ImageInfo, Option<gfx_hal::command::ClearValue>)>,
    frames_in_flight: u32,
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
            frames_in_flight: 3,
        }
    }

    /// Create new buffer owned by graph.
    pub fn create_buffer(&mut self, size: u64) -> BufferId {
        self.buffers.push(BufferInfo {
            size,
            usage: gfx_hal::buffer::Usage::empty(),
        });
        BufferId(self.buffers.len() - 1)
    }

    /// Create new image owned by graph.
    pub fn create_image(
        &mut self,
        kind: gfx_hal::image::Kind,
        levels: gfx_hal::image::Level,
        format: gfx_hal::format::Format,
        clear: Option<gfx_hal::command::ClearValue>,
    ) -> ImageId {
        self.images.push((
            ImageInfo {
                kind,
                levels,
                format,
                tiling: gfx_hal::image::Tiling::Optimal,
                view_caps: gfx_hal::image::ViewCapabilities::empty(),
                usage: gfx_hal::image::Usage::empty(),
            },
            clear,
        ));
        ImageId(self.images.len() - 1)
    }

    /// Add node to the graph.
    pub fn add_node<N: NodeBuilder<B, T> + 'static>(&mut self, builder: N) -> NodeId {
        self.nodes.push(Box::new(builder));
        NodeId(self.nodes.len() - 1)
    }

    /// Choose number of frames in flight for the graph
    pub fn with_frames_in_flight(mut self, frames_in_flight: u32) -> Self {
        self.frames_in_flight = frames_in_flight;
        self
    }

    /// Build `Graph`.
    ///
    /// # Parameters
    ///
    /// `frames`        - maximum number of frames `Graph` will render simultaneously.
    ///
    /// `families`      - `Iterator` of `B::QueueFamily`s.
    ///
    /// `device`    - `Device<B>` implementation. `Device<B>` or wrapper.
    ///
    /// `aux`       - auxiliary data that `Node`s use.
    pub fn build(
        self,
        factory: &mut Factory<B>,
        families: &mut Families<B>,
        aux: &T,
    ) -> Result<Graph<B, T>, failure::Error> {
        log::trace!("Schedule nodes execution");
        let chain_nodes: Vec<chain::Node> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, b)| make_chain_node(&**b, i, factory, families))
            .collect();

        let chains = chain::collect(chain_nodes, |id| {
            families.family_by_index(id.0).as_slice().len()
        });
        log::trace!("Scheduled nodes execution {:#?}", chains);

        let mut ctx = GraphContext::alloc(factory, &chains, &self.buffers, &self.images)?;

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
                    let node = build_node(
                        &mut ctx,
                        builder,
                        factory,
                        families.family_by_index_mut(family.id().0),
                        queue.id().index(),
                        aux,
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
            device: factory.device().id(),
            ctx,
            nodes: built_nodes
                .into_iter()
                .map(Option::unwrap)
                .map(|(node, qid)| GraphNode {
                    node,
                    queue: (qid.family().0, qid.index()),
                })
                .collect(),
            schedule,
            semaphores,
            inflight: self.frames_in_flight,
            frames: Frames::new(),
            fences: Vec::new(),
        })
    }
}

fn build_node<'a, B: Backend, T: ?Sized>(
    ctx: &mut GraphContext<B>,
    builder: Box<dyn NodeBuilder<B, T>>,
    factory: &mut Factory<B>,
    family: &mut rendy_command::Family<B>,
    queue: usize,
    aux: &T,
    chains: &chain::Chains,
    submission: &chain::Submission<chain::SyncData<usize, usize>>,
) -> Result<Box<dyn DynNode<B, T>>, failure::Error> {
    let mut buffer_ids: Vec<_> = builder.buffers().into_iter().map(|(id, _)| id).collect();
    buffer_ids.sort();
    buffer_ids.dedup();

    let buffers: Vec<_> = buffer_ids
        .into_iter()
        .map(|id| {
            let chain_id = chain::Id(id.0);
            let sync = submission.sync();
            let buffer = ctx
                .get_buffer(id)
                .expect("Buffer referenced from at least one node must be instantiated");
            NodeBuffer {
                id,
                range: 0..buffer.size(),
                acquire: sync.acquire.buffers.get(&chain_id).map(
                    |chain::Barrier { states, families }| BufferBarrier {
                        states: states.start.0..states.end.0,
                        stages: states.start.2..states.end.2,
                        families: families.clone(),
                    },
                ),
                release: sync.release.buffers.get(&chain_id).map(
                    |chain::Barrier { states, families }| BufferBarrier {
                        states: states.start.0..states.end.0,
                        stages: states.start.2..states.end.2,
                        families: families.clone(),
                    },
                ),
            }
        })
        .collect();

    let mut image_ids: Vec<_> = builder.images().into_iter().map(|(id, _)| id).collect();
    image_ids.sort();
    image_ids.dedup();

    let images: Vec<_> = image_ids
        .into_iter()
        .map(|id| {
            let chain_id = chain::Id(id.0);
            let sync = submission.sync();
            let link = submission.image_link_index(chain_id);
            let (image, clear) = ctx
                .get_image_with_clear(id)
                .expect("Image referenced from at least one node must be instantiated");
            NodeImage {
                id,
                range: gfx_hal::image::SubresourceRange {
                    aspects: image.format().surface_desc().aspects,
                    levels: 0..image.levels(),
                    layers: 0..image.layers(),
                },
                layout: chains.images[&chain_id].links()[link]
                    .submission_state(submission.id())
                    .layout,
                clear: if link == 0 { clear } else { None },
                acquire: sync.acquire.images.get(&chain_id).map(
                    |chain::Barrier { states, families }| ImageBarrier {
                        states: (states.start.0, states.start.1)..(states.end.0, states.end.1),
                        stages: states.start.2..states.end.2,
                        families: families.clone(),
                    },
                ),
                release: sync.release.images.get(&chain_id).map(
                    |chain::Barrier { states, families }| ImageBarrier {
                        states: (states.start.0, states.start.1)..(states.end.0, states.end.1),
                        stages: states.start.2..states.end.2,
                        families: families.clone(),
                    },
                ),
            }
        })
        .collect();
    builder.build(ctx, factory, family, queue, aux, buffers, images)
}

fn make_chain_node<B, T>(
    builder: &dyn NodeBuilder<B, T>,
    id: usize,
    factory: &mut Factory<B>,
    families: &Families<B>,
) -> chain::Node
where
    B: Backend,
    T: ?Sized,
{
    let buffers = builder.buffers();
    let images = builder.images();
    chain::Node {
        id,
        family: QueueFamilyId(builder.family(factory, families.as_slice()).unwrap().index),
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
