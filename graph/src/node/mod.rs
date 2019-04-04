//! Defines node - building block for framegraph.
//!

pub mod present;
pub mod render;

use {
    crate::{
        command::{Capability, Family, FamilyId, Fence, Queue, Submission, Submittable, Supports},
        factory::Factory,
        frame::Frames,
        graph::GraphContext,
        BufferId, ImageId, NodeId,
    },
    gfx_hal::{queue::QueueFamilyId, Backend},
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
    pub families: Option<std::ops::Range<QueueFamilyId>>,
}

/// Buffer shared between nodes.
///
/// If Node doesn't actually use the buffer it can merge acquire and release barriers into one.
/// TODO: Make merge function.
#[derive(Clone, Debug)]
pub struct NodeBuffer {
    /// Id of the buffer.
    pub id: BufferId,

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
    pub families: Option<std::ops::Range<QueueFamilyId>>,
}

/// Image shared between nodes.
#[derive(Clone, Debug)]
pub struct NodeImage {
    /// Id of the image.
    pub id: ImageId,

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
pub trait NodeSubmittable<'a, B: Backend> {
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
pub trait Node<B: Backend, T: ?Sized>:
    for<'a> NodeSubmittable<'a, B> + std::fmt::Debug + Sized + Sync + Send + 'static
{
    /// Capability required by node.
    /// Graph will execute this node on command queue that supports this capability level.
    type Capability: Capability;

    /// Description type to instantiate the node.
    type Desc: NodeDesc<B, T, Node = Self>;

    /// Desc creation.
    /// Convenient method if `Self::Desc` implements `Default`.
    fn desc() -> Self::Desc
    where
        Self::Desc: Default,
    {
        Default::default()
    }

    /// Builder creation.
    fn builder() -> DescBuilder<B, T, Self::Desc>
    where
        Self::Desc: Default,
    {
        Self::desc().builder()
    }

    /// Record commands required by node.
    /// Returned submits are guaranteed to be submitted within specified frame.
    fn run<'a>(
        &'a mut self,
        ctx: &GraphContext<B>,
        factory: &Factory<B>,
        aux: &T,
        frames: &'a Frames<B>,
    ) -> <Self as NodeSubmittable<'a, B>>::Submittables;

    /// Dispose of the node.
    ///
    /// # Safety
    ///
    /// Must be called after waiting for device idle.
    unsafe fn dispose(self, factory: &mut Factory<B>, aux: &T);
}

/// Description of the node.
/// Implementation of the builder type provide framegraph with static information about node
/// that is used for building the node.
pub trait NodeDesc<B: Backend, T: ?Sized>: std::fmt::Debug + Sized + 'static {
    /// Node this builder builds.
    type Node: Node<B, T>;

    /// Make node builder.
    fn builder(self) -> DescBuilder<B, T, Self> {
        DescBuilder {
            desc: self,
            buffers: Vec::new(),
            images: Vec::new(),
            dependencies: Vec::new(),
            marker: std::marker::PhantomData,
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
        self,
        ctx: &mut GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        queue: usize,
        aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Self::Node, failure::Error>;
}

/// Trait-object safe `Node`.
pub trait DynNode<B: Backend, T: ?Sized>: std::fmt::Debug + Sync + Send {
    /// Record commands required by node.
    /// Recorded buffers go into `submits`.
    unsafe fn run<'a>(
        &mut self,
        ctx: &GraphContext<B>,
        factory: &Factory<B>,
        queue: &mut Queue<B>,
        aux: &T,
        frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&mut Fence<B>>,
    );

    /// Dispose of the node.
    ///
    /// # Safety
    ///
    /// Must be called after waiting for device idle.
    unsafe fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &T);
}

impl<B, T, N> DynNode<B, T> for (N,)
where
    B: Backend,
    T: ?Sized,
    N: Node<B, T>,
{
    unsafe fn run<'a>(
        &mut self,
        ctx: &GraphContext<B>,
        factory: &Factory<B>,
        queue: &mut Queue<B>,
        aux: &T,
        frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, gfx_hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&mut Fence<B>>,
    ) {
        let submittables = Node::run(&mut self.0, ctx, factory, aux, frames);
        queue.submit(
            Some(
                Submission::new()
                    .submits(submittables)
                    .wait(waits.iter().cloned())
                    .signal(signals.iter().cloned()),
            ),
            fence,
        )
    }

    unsafe fn dispose(self: Box<Self>, factory: &mut Factory<B>, aux: &T) {
        N::dispose(self.0, factory, aux);
    }
}

/// Dynamic ode builder that emits `DynNode`.
pub trait NodeBuilder<B: Backend, T: ?Sized>: std::fmt::Debug {
    /// Pick family for this node to be executed onto.
    fn family(&self, factory: &mut Factory<B>, families: &[Family<B>]) -> Option<FamilyId>;

    /// Get buffer accessed by the node.
    fn buffers(&self) -> Vec<(BufferId, BufferAccess)>;

    /// Get images accessed by the node.
    fn images(&self) -> Vec<(ImageId, ImageAccess)>;

    /// Indices of nodes this one dependes on.
    fn dependencies(&self) -> Vec<NodeId>;

    /// Build node.
    fn build<'a>(
        self: Box<Self>,
        ctx: &mut GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        queue: usize,
        aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Box<dyn DynNode<B, T>>, failure::Error>;
}

/// Builder for the node.
#[derive(derivative::Derivative)]
#[derivative(Debug(bound = "N: std::fmt::Debug"))]
pub struct DescBuilder<B: Backend, T: ?Sized, N> {
    desc: N,
    buffers: Vec<BufferId>,
    images: Vec<ImageId>,
    dependencies: Vec<NodeId>,
    marker: std::marker::PhantomData<fn(B, &T)>,
}

impl<B, T, N> DescBuilder<B, T, N>
where
    B: Backend,
    T: ?Sized,
{
    /// Add buffer to the node.
    /// This method must be called for each buffer node uses.
    pub fn add_buffer(&mut self, buffer: BufferId) -> &mut Self {
        self.buffers.push(buffer);
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
    pub fn add_image(&mut self, image: ImageId) -> &mut Self {
        self.images.push(image);
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
    pub fn add_dependency(&mut self, dependency: NodeId) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: NodeId) -> Self {
        self.add_dependency(dependency);
        self
    }
}

impl<B, T, N> NodeBuilder<B, T> for DescBuilder<B, T, N>
where
    B: Backend,
    T: ?Sized,
    N: NodeDesc<B, T>,
{
    fn family(&self, _factory: &mut Factory<B>, families: &[Family<B>]) -> Option<FamilyId> {
        families
            .iter()
            .find(|family| {
                Supports::<<N::Node as Node<B, T>>::Capability>::supports(&family.capability())
                    .is_some()
            })
            .map(|family| family.id())
    }

    fn buffers(&self) -> Vec<(BufferId, BufferAccess)> {
        let desc_buffers = self.desc.buffers();
        assert_eq!(self.buffers.len(), desc_buffers.len());

        self.buffers.iter().cloned().zip(desc_buffers).collect()
    }

    fn images(&self) -> Vec<(ImageId, ImageAccess)> {
        let desc_images = self.desc.images();
        assert_eq!(self.images.len(), desc_images.len());

        self.images.iter().cloned().zip(desc_images).collect()
    }

    fn dependencies(&self) -> Vec<NodeId> {
        self.dependencies.clone()
    }

    fn build<'a>(
        self: Box<Self>,
        ctx: &mut GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        queue: usize,
        aux: &T,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Box<dyn DynNode<B, T>>, failure::Error> {
        Ok(Box::new((self.desc.build(
            ctx, factory, family, queue, aux, buffers, images,
        )?,)))
    }
}

/// Convert graph barriers into gfx barriers.
pub fn gfx_acquire_barriers<'a, 'b, B: Backend>(
    ctx: &'a GraphContext<B>,
    buffers: impl IntoIterator<Item = &'b NodeBuffer>,
    images: impl IntoIterator<Item = &'b NodeImage>,
) -> (
    std::ops::Range<gfx_hal::pso::PipelineStage>,
    Vec<gfx_hal::memory::Barrier<'a, B>>,
) {
    let mut bstart = gfx_hal::pso::PipelineStage::empty();
    let mut bend = gfx_hal::pso::PipelineStage::empty();

    let mut istart = gfx_hal::pso::PipelineStage::empty();
    let mut iend = gfx_hal::pso::PipelineStage::empty();

    let barriers: Vec<gfx_hal::memory::Barrier<'_, B>> = buffers
        .into_iter()
        .filter_map(|buffer| {
            buffer.acquire.as_ref().map(|acquire| {
                bstart |= acquire.stages.start;
                bend |= acquire.stages.end;

                gfx_hal::memory::Barrier::Buffer {
                    states: acquire.states.clone(),
                    families: acquire.families.clone(),
                    target: ctx
                        .get_buffer(buffer.id)
                        .expect("Buffer does not exist")
                        .raw(),
                    range: Some(buffer.range.start)..Some(buffer.range.end),
                }
            })
        })
        .chain(images.into_iter().filter_map(|image| {
            image.acquire.as_ref().map(|acquire| {
                istart |= acquire.stages.start;
                iend |= acquire.stages.end;

                gfx_hal::memory::Barrier::Image {
                    states: acquire.states.clone(),
                    families: acquire.families.clone(),
                    target: ctx.get_image(image.id).expect("Image does not exist").raw(),
                    range: image.range.clone(),
                }
            })
        }))
        .collect();

    (bstart | istart..bend | iend, barriers)
}

/// Convert graph barriers into gfx barriers.
pub fn gfx_release_barriers<'a, B: Backend>(
    ctx: &'a GraphContext<B>,
    buffers: impl IntoIterator<Item = &'a NodeBuffer>,
    images: impl IntoIterator<Item = &'a NodeImage>,
) -> (
    std::ops::Range<gfx_hal::pso::PipelineStage>,
    Vec<gfx_hal::memory::Barrier<'a, B>>,
) {
    let mut bstart = gfx_hal::pso::PipelineStage::empty();
    let mut bend = gfx_hal::pso::PipelineStage::empty();

    let mut istart = gfx_hal::pso::PipelineStage::empty();
    let mut iend = gfx_hal::pso::PipelineStage::empty();

    let barriers: Vec<gfx_hal::memory::Barrier<'_, B>> = buffers
        .into_iter()
        .filter_map(|buffer| {
            buffer.release.as_ref().map(|release| {
                bstart |= release.stages.start;
                bend |= release.stages.end;

                gfx_hal::memory::Barrier::Buffer {
                    states: release.states.clone(),
                    families: release.families.clone(),
                    target: ctx
                        .get_buffer(buffer.id)
                        .expect("Buffer does not exist")
                        .raw(),
                    range: Some(buffer.range.start)..Some(buffer.range.end),
                }
            })
        })
        .chain(images.into_iter().filter_map(|image| {
            image.release.as_ref().map(|release| {
                istart |= release.stages.start;
                iend |= release.stages.end;

                gfx_hal::memory::Barrier::Image {
                    states: release.states.clone(),
                    families: release.families.clone(),
                    target: ctx.get_image(image.id).expect("Image does not exist").raw(),
                    range: image.range.clone(),
                }
            })
        }))
        .collect();

    (bstart | istart..bend | iend, barriers)
}

/// Check if backend is metal.
#[cfg(feature = "metal")]
pub fn is_metal<B: Backend>() -> bool {
    std::any::TypeId::of::<B>() == std::any::TypeId::of::<gfx_backend_metal::Backend>()
}

/// Check if backend is metal.
#[cfg(not(feature = "metal"))]
pub fn is_metal<B: Backend>() -> bool {
    false
}
