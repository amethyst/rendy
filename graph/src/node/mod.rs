//! Defines node - building block for framegraph.
//!

pub mod render;
pub mod present;

use crate::{
    chain,
    command::{Capability, Family, Supports, Submission, Submittable},
    factory::Factory,
    frame::Frames,
    resource::{Buffer, Image},
    BufferId,
    ImageId,
    NodeId,
};

/// Buffer access node will perform.
/// Node must not perform any access to the buffer not specified in `access`.
/// All access must be between logically first and last `stages`.
#[derive(Clone, Copy, Debug)]
pub struct BufferAccess {
    /// Access flags.
    pub access: gfx_hal::buffer::Access,

    /// Intended usage flags for buffer.
    /// TODO: Could derive from access?
    pub usage: gfx_hal::buffer::Usage,

    /// Pipeline stages at which buffer is accessd.
    pub stages: gfx_hal::pso::PipelineStage,
}

/// Buffer pipeline barrier.
#[derive(Clone, Debug)]
pub struct BufferBarrier {
    /// State transition for the buffer.
    pub states: std::ops::Range<gfx_hal::buffer::State>,

    /// Stages at which buffer is accessd.
    pub stages: std::ops::Range<gfx_hal::pso::PipelineStage>,

    /// Transfer between families.
    pub families: Option<std::ops::Range<gfx_hal::queue::QueueFamilyId>>,
}


/// Buffer shared between nodes.
/// 
/// If Node doesn't actually use the buffer it can merge acquire and release barriers into one.
/// TODO: Make merge function.
#[derive(Debug)]
pub struct NodeBuffer<'a, B: gfx_hal::Backend> {
    /// Buffer reference.
    pub buffer: &'a mut Buffer<B>,

    /// Region of the buffer that is the transient resource.
    pub range: std::ops::Range<u64>,

    /// Acquire barrier.
    /// Node implementation must insert it before first command that uses the buffer.
    /// Barrier must be inserted even if this node doesn't use the buffer.
    pub acquire: Option<BufferBarrier>,

    /// Release barrier.
    /// Node implementation must insert it after last command that uses the buffer.
    /// Barrier must be inserted even if this node doesn't use the buffer.
    pub release: Option<BufferBarrier>,
}


/// Image access node wants to perform.
#[derive(Clone, Copy, Debug)]
pub struct ImageAccess {
    /// Access flags.
    pub access: gfx_hal::image::Access,

    /// Intended usage flags for image.
    /// TODO: Could derive from access?
    pub usage: gfx_hal::image::Usage,

    /// Preferred layout for access.
    /// Actual layout will be reported int `NodeImage`.
    /// Actual layout is guaranteed to support same operations.
    /// TODO: Could derive from access?
    pub layout: gfx_hal::image::Layout,

    /// Pipeline stages at which image is accessd.
    pub stages: gfx_hal::pso::PipelineStage,
}

/// Image pipeline barrier.
/// Node implementation must insert it before first command that uses the image.
/// Barrier must be inserted even if this node doesn't use the image.
#[derive(Clone, Debug)]
pub struct ImageBarrier {
    /// State transition for the image.
    pub states: std::ops::Range<gfx_hal::image::State>,

    /// Stages at which image is accessd.
    pub stages: std::ops::Range<gfx_hal::pso::PipelineStage>,

    /// Transfer between families.
    pub families: Option<std::ops::Range<gfx_hal::queue::QueueFamilyId>>,
}

/// Image shared between nodes.
#[derive(Debug)]
pub struct NodeImage<'a, B: gfx_hal::Backend> {
    /// Image reference.
    pub image: &'a mut Image<B>,

    /// Region of the image that is the transient resource.
    pub range: gfx_hal::image::SubresourceRange,

    /// Image state for node.
    pub layout: gfx_hal::image::Layout,

    /// Specify that node should clear image to this value.
    pub clear: Option<gfx_hal::command::ClearValue>,

    /// Acquire barrier.
    /// Node implementation must insert it before first command that uses the image.
    /// Barrier must be inserted even if this node doesn't use the image.
    pub acquire: Option<ImageBarrier>,

    /// Release barrier.
    /// Node implementation must insert it after last command that uses the image.
    /// Barrier must be inserted even if this node doesn't use the image.
    pub release: Option<ImageBarrier>,
}

/// NodeSubmittable
pub trait NodeSubmittable<'a, B: gfx_hal::Backend> {
    /// Submittable type returned from `Node`.
    type Submittable: Submittable<B> + 'a;

    /// Iterator over submittables returned from `Node`.
    type Submittables: IntoIterator<Item = Self::Submittable>;
}

/// The node is building block of the framegraph.
/// Node defines set of resources and operations to perform over them.
/// Read-only data for operations comes from auxiliary data source `T`.
///
/// # Parameters
///
/// `B` - backend type.
/// `T` - auxiliary data type.
///
pub trait Node<B: gfx_hal::Backend, T: ?Sized>:
    for<'a> NodeSubmittable<'a, B> + std::fmt::Debug + Sized + Sync + Send + 'static
{
    /// Capability required by node.
    /// Graph will execute this node on command queue that supports this capability level.
    type Capability: Capability;

    /// Description type to instantiate the node.
    type Desc: NodeDesc<B, T, Node = Self>;

    /// Desc creation.
    /// Convenient method if builder implements `Default`.
    fn desc() -> Self::Desc
    where
        Self::Desc: Default,
    {
        Default::default()
    }

    /// Builder creation.
    fn builder() -> NodeBuilder<B, T>
    where
        Self::Desc: Default,
    {
        Self::desc().builder()
    }

    /// Record commands required by node.
    /// Returned submits are guaranteed to be submitted within specified frame.
    fn run<'a>(
        &'a mut self,
        factory: &mut Factory<B>,
        aux: &mut T,
        frames: &'a Frames<B>,
    ) -> <Self as NodeSubmittable<'a, B>>::Submittables;

    /// Dispose of the node.
    /// 
    /// # Safety
    /// 
    /// Must be called after waiting for device idle.
    unsafe fn dispose(self, factory: &mut Factory<B>, aux: &mut T);
}

/// Builder of the node.
/// Implementation of the builder type provide framegraph with static information about node
/// that is used for building the node.
pub trait NodeDesc<B: gfx_hal::Backend, T: ?Sized>: std::fmt::Debug + Sized + 'static {
    /// Node this builder builds.
    type Node: Node<B, T>;

    /// Make node builder.
    fn builder(self) -> NodeBuilder<B, T> {
        NodeBuilder {
            desc: Box::new((self,)),
            buffers: Vec::new(),
            images: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// Get set or buffer resources the node uses.
    fn buffers(&self) -> Vec<BufferAccess> {
        Vec::new()
    }

    /// Get set or image resources the node uses.
    fn images(&self) -> Vec<ImageAccess> {
        Vec::new()
    }

    /// Build the node.
    ///
    /// # Parameters
    ///
    /// `factory`    - factory instance.
    /// `aux`       - auxiliary data.
    /// `family`    - id of the family this node will be executed on.
    /// `resources` - set of transient resources managed by graph.
    ///               with barriers required for interface resources.
    ///
    fn build<'a>(
        &self,
        factory: &mut Factory<B>,
        aux: &mut T,
        family: gfx_hal::queue::QueueFamilyId,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
    ) -> Result<Self::Node, failure::Error>;
}

/// Trait-object safe `Node`.
pub trait AnyNode<B: gfx_hal::Backend, T: ?Sized>:
    std::fmt::Debug + Sync + Send
{
    /// Record commands required by node.
    /// Recorded buffers go into `submits`.
    unsafe fn run<'a>(
        &mut self,
        factory: &mut Factory<B>,
        aux: &mut T,
        frames: &Frames<B>,
        qid: chain::QueueId,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&B::Fence>,
    );

    /// Dispose of the node.
    /// 
    /// # Safety
    /// 
    /// Must be called after waiting for device idle.
    unsafe fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &mut T);
}

impl<B, T, N> AnyNode<B, T> for (N,)
where
    B: gfx_hal::Backend,
    T: ?Sized,
    N: Node<B, T>,
{
    unsafe fn run<'a>(
        &mut self,
        factory: &mut Factory<B>,
        aux: &mut T,
        frames: &Frames<B>,
        qid: chain::QueueId,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&B::Fence>,
    ) {
        let submittables = Node::run(&mut self.0, factory, aux, frames);
        factory.family_mut(qid.family()).submit(
            qid.index(),
            Some(Submission {
                waits: waits.iter().cloned(),
                signals: signals.iter().cloned(),
                submits: submittables,
            }),
            fence,
        )
    }

    unsafe fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &mut T) {
        N::dispose(self.0, factory, aux);
    }
}

/// Trait-object safe `NodeDesc`.
pub trait AnyNodeDesc<B: gfx_hal::Backend, T: ?Sized>: std::fmt::Debug {
    /// Find family suitable for the node.
    fn family(&self, families: &[Family<B>]) -> Option<gfx_hal::queue::QueueFamilyId>;

    /// Get buffer resource states.
    fn buffers(&self) -> Vec<BufferAccess> { Vec::new() }

    /// Get image resource states.
    fn images(&self) -> Vec<ImageAccess> { Vec::new() }

    /// Build the node.
    fn build<'a>(
        self: Box<Self>,
        factory: &mut Factory<B>,
        aux: &mut T,
        family: gfx_hal::queue::QueueFamilyId,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
    ) -> Result<Box<dyn AnyNode<B, T>>, failure::Error>;

    /// Make node builder.
    fn builder(self) -> NodeBuilder<B, T>
    where
        Self: Sized + 'static,
    {
        NodeBuilder {
            desc: Box::new(self),
            buffers: Vec::new(),
            images: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

impl<B, T, N> AnyNodeDesc<B, T> for (N,)
where
    B: gfx_hal::Backend,
    T: ?Sized,
    N: NodeDesc<B, T>,
{
    fn family(&self, families: &[Family<B>]) -> Option<gfx_hal::queue::QueueFamilyId> {
        families
            .iter()
            .find(|family| {
                Supports::<<N::Node as Node<B, T>>::Capability>::supports(&family.capability())
                    .is_some()
            }).map(|family| family.index())
    }

    fn buffers(&self) -> Vec<BufferAccess> {
        N::buffers(&self.0)
    }

    fn images(&self) -> Vec<ImageAccess> {
        N::images(&self.0)
    }

    fn build<'a>(
        self: Box<Self>,
        factory: &mut Factory<B>,
        aux: &mut T,
        family: gfx_hal::queue::QueueFamilyId,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
    ) -> Result<Box<dyn AnyNode<B, T>>, failure::Error> {
        let node = NodeDesc::build(
            &self.0,
            factory,
            aux,
            family,
            buffers,
            images,
        )?;
        Ok(Box::new((node,)))
    }
}

/// Builder for the node.
#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
pub struct NodeBuilder<B: gfx_hal::Backend, T: ?Sized> {
    pub(crate) desc: Box<dyn AnyNodeDesc<B, T>>,
    pub(crate) buffers: Vec<BufferId>,
    pub(crate) images: Vec<ImageId>,
    pub(crate) dependencies: Vec<usize>,
}

impl<B, T> NodeBuilder<B, T>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    /// Add buffer to the node.
    /// This method must be called for each buffer node uses.
    pub fn add_buffer(&mut self, buffer: BufferId) -> &mut Self {
        self.buffers.push(buffer);
        self
    }

    /// Add image to the node.
    /// This method must be called for each image node uses.
    pub fn add_image(&mut self, image: ImageId) -> &mut Self {
        self.images.push(image);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn add_dependency(&mut self, dependency: NodeId) -> &mut Self {
        self.dependencies.push(dependency.0);
        self
    }

    /// Add buffer to the node.
    /// This method must be called for each buffer node uses.
    pub fn with_buffer(mut self, buffer: BufferId) -> Self {
        self.add_buffer(buffer);
        self
    }

    /// Add image to the node.
    /// This method must be called for each image node uses.
    pub fn with_image(mut self, image: ImageId) -> Self {
        self.add_image(image);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: NodeId) -> Self {
        self.add_dependency(dependency);
        self
    }

    pub(crate) fn chain(&self, id: usize, factory: &Factory<B>, buffers: usize) -> chain::Node {
        let desc_buffers = self.desc.buffers();
        assert_eq!(self.buffers.len(), desc_buffers.len());

        let desc_images = self.desc.images();
        assert_eq!(self.images.len(), desc_images.len());

        chain::Node {
            id,
            family: self.desc.family(factory.families()).unwrap(),
            dependencies: self.dependencies.clone(),
            buffers: self
                .buffers
                .iter()
                .map(|id| chain::Id(id.0))
                .zip(desc_buffers)
                .map(|(id, access)| {
                    (id, chain::BufferState {
                        access: access.access,
                        stages: access.stages,
                        layout: (),
                        usage: access.usage,
                    })
                })
                .collect(),
            images: self
                .images
                .iter()
                .map(|id| chain::Id(id.0 + buffers))
                .zip(desc_images)
                .map(|(id, access)| {
                    (id, chain::ImageState {
                        access: access.access,
                        stages: access.stages,
                        layout: access.layout,
                        usage: access.usage,
                    })
                })
                .collect(),
        }
    }

    /// Build node from this.
    #[allow(unused)]
    pub(crate) fn build<'a>(
        self,
        factory: &mut Factory<B>,
        aux: &mut T,
        family: gfx_hal::queue::QueueFamilyId,
        buffers: &mut [Option<Buffer<B>>],
        images: &mut [Option<(Image<B>, Option<gfx_hal::command::ClearValue>)>],
        chains: &chain::Chains,
        submission: &chain::Submission<chain::SyncData<usize, usize>>,
    ) -> Result<Box<dyn AnyNode<B, T>>, failure::Error> {
        let buffers_len = buffers.len();
        self.desc.build(
            factory,
            aux,
            family,
            &mut self.buffers.iter().zip(buffers).map(|(&BufferId(index), resource)| {
                let id = chain::Id(index);
                let sync = submission.sync();
                let buffer = resource.as_mut().expect("Buffer referenced from at least one node must be instantiated");
                NodeBuffer {
                    range: 0 .. buffer.size(),
                    acquire: sync.acquire.buffers.get(&id).map(|chain::Barrier { states, families }| BufferBarrier {
                        states: states.start.0 .. states.end.0,
                        stages: states.start.2 .. states.end.2,
                        families: families.clone(),
                    }),
                    release: sync.release.buffers.get(&id).map(|chain::Barrier { states, families }| BufferBarrier {
                        states: states.start.0 .. states.end.0,
                        stages: states.start.2 .. states.end.2,
                        families: families.clone(),
                    }),
                    buffer,
                }
            }).collect::<Vec<_>>(),
            &mut self.images.iter().zip(images).map(|(&ImageId(index), resource)| {
                let id = chain::Id(index + buffers_len);
                let sync = submission.sync();
                let (image, clear) = resource.as_mut().expect("Image referenced from at least one node must be instantiated");
                NodeImage {
                    range: gfx_hal::image::SubresourceRange {
                        aspects: image.format().surface_desc().aspects,
                        levels: 0 .. image.levels(),
                        layers: 0 .. image.layers(),
                    },
                    layout: chains.images[&id].links()[submission.resource_link_index(id)].submission_state(submission.id()).layout,
                    clear: if submission.resource_link_index(id) == 0 {
                        *clear
                    } else {
                        None
                    },
                    acquire: sync.acquire.images.get(&id).map(|chain::Barrier { states, families }| ImageBarrier {
                        states: (states.start.0, states.start.1) .. (states.end.0, states.end.1),
                        stages: states.start.2 .. states.end.2,
                        families: families.clone(),
                    }),
                    release: sync.release.images.get(&id).map(|chain::Barrier { states, families }| ImageBarrier {
                        states: (states.start.0, states.start.1) .. (states.end.0, states.end.1),
                        stages: states.start.2 .. states.end.2,
                        families: families.clone(),
                    }),
                    image,
                }
            }).collect::<Vec<_>>(),
        )
    }
}

/// Convert graph barriers into gfx barriers.
pub fn gfx_acquire_barriers<'a, B: gfx_hal::Backend>(buffers: impl IntoIterator<Item = &'a NodeBuffer<'a, B>>, images: impl IntoIterator<Item = &'a NodeImage<'a, B>>) -> (std::ops::Range<gfx_hal::pso::PipelineStage>, Vec<gfx_hal::memory::Barrier<'a, B>>) {
    let mut bstart = gfx_hal::pso::PipelineStage::empty();
    let mut bend = gfx_hal::pso::PipelineStage::empty();

    let mut istart = gfx_hal::pso::PipelineStage::empty();
    let mut iend = gfx_hal::pso::PipelineStage::empty();

    let barriers: Vec<gfx_hal::memory::Barrier<'_, B>> = buffers.into_iter().filter_map(|buffer| {
        if let Some(acquire) = &buffer.acquire {
            bstart |= acquire.stages.start;
            bend |= acquire.stages.end;

            Some(gfx_hal::memory::Barrier::Buffer {
                states: acquire.states.clone(),
                families: acquire.families.clone(),
                target: buffer.buffer.raw(),
                // range: buffer.range.clone(),
            })
        } else {
            None
        }
    }).chain(images.into_iter().filter_map(|image| {
        if let Some(acquire) = &image.acquire {
            istart |= acquire.stages.start;
            iend |= acquire.stages.end;

            Some(gfx_hal::memory::Barrier::Image {
                states: acquire.states.clone(),
                families: acquire.families.clone(),
                target: image.image.raw(),
                range: image.range.clone(),
            })
        } else {
            None
        }
    })).collect();

    (bstart|istart .. bend|iend, barriers)
}

/// Convert graph barriers into gfx barriers.
pub fn gfx_release_barriers<'a, B: gfx_hal::Backend>(buffers: impl IntoIterator<Item = &'a NodeBuffer<'a, B>>, images: impl IntoIterator<Item = &'a NodeImage<'a, B>>) -> (std::ops::Range<gfx_hal::pso::PipelineStage>, Vec<gfx_hal::memory::Barrier<'a, B>>) {
    let mut bstart = gfx_hal::pso::PipelineStage::empty();
    let mut bend = gfx_hal::pso::PipelineStage::empty();

    let mut istart = gfx_hal::pso::PipelineStage::empty();
    let mut iend = gfx_hal::pso::PipelineStage::empty();

    let barriers: Vec<gfx_hal::memory::Barrier<'_, B>> = buffers.into_iter().filter_map(|buffer| {
        if let Some(release) = &buffer.release {
            bstart |= release.stages.start;
            bend |= release.stages.end;

            Some(gfx_hal::memory::Barrier::Buffer {
                states: release.states.clone(),
                families: release.families.clone(),
                target: buffer.buffer.raw(),
                // range: buffer.range.clone(),
            })
        } else {
            None
        }
    }).chain(images.into_iter().filter_map(|image| {
        if let Some(release) = &image.release {
            istart |= release.stages.start;
            iend |= release.stages.end;

            Some(gfx_hal::memory::Barrier::Image {
                states: release.states.clone(),
                families: release.families.clone(),
                target: image.image.raw(),
                range: image.range.clone(),
            })
        } else {
            None
        }
    })).collect();

    (bstart|istart .. bend|iend, barriers)
}
