use chain::{
    access::AccessFlags, node::State, resource::{Buffer, Image}, Id,
};
use command::{
    buffer::Submit, capability::{Capability, CapabilityFlags}, device::Device, family::FamilyId,
    frame::{Frame, FrameBound}, pool::FramePool,
};
use resource::{buffer, image};

pub trait FrameBoundSubmits<'a, D: Device + ?Sized> {
    type Submits: IntoIterator<Item = Submit<FrameBound<'a, D::Fence, D::Submit>>>;
}

/// The node is building block of the framegraph.
/// Node defines set of resources and operations to perform over them.
/// Read-only data for operations comes from auxiliary data source `T`.
///
/// # Parameters
///
/// `D` - device type.
/// `T` - auxiliary data type.
///
pub trait Node<D: Device + ?Sized, T: ?Sized>:
    for<'a> FrameBoundSubmits<'a, D> + Sized + 'static
{
    /// Capability required by node.
    /// Graph will execute this node on command queue that supports this capability level.
    type Capability: Capability;

    /// Description type to instantiate the node.
    type Desc: NodeDesc<D, T, Node = Self>;

    /// Builder creation.
    /// Convenient method if builder implements `Default`.
    fn desc() -> Self::Desc
    where
        Self::Desc: Default,
    {
        Default::default()
    }

    /// Record commands required by node.
    /// Returned submits are guaranteed to be submitted within specified frame.
    fn run<'a>(
        &mut self,
        device: &D,
        aux: &T,
        frame: &'a Frame<D::Fence>,
    ) -> <Self as FrameBoundSubmits<'a, D>>::Submits;
}

/// Resources wrapper.
/// Wraps resources requested by the node.
/// This wrapper guarantees that lifetime of resources is bound to the node lifetime.
/// Also it automatically inserts synchronization required to make access declared by node correct.
pub struct Resources<'a, B: 'a, I: 'a> {
    buffers: Vec<&'a B>,
    images: Vec<&'a I>,
    barriers: Barriers,
}

pub struct Barriers;

/// Builder of the node.
/// Implementation of the builder type provide framegraph with static information about node
/// that is used for building the node.
pub trait NodeDesc<D: Device + ?Sized, T: ?Sized>: Sized + 'static {
    /// Node this builder builds.
    type Node: Node<D, T>;

    /// Capability required by node.
    /// Graph will execute this node on command queue that supports this capability level.

    /// Get set or buffer resources the node uses.
    fn buffers(&self) -> Vec<State<Buffer>> {
        Vec::new()
    }

    /// Get set or image resources the node uses.
    fn images(&self) -> Vec<State<Image>> {
        Vec::new()
    }

    /// Build the node.
    ///
    /// # Parameters
    ///
    /// `device`    - device instance.
    /// `aux`       - auxiliary data.
    /// `family`    - id of the family this node will be executed on.
    /// `resources` - set of transient resources managed by graph.
    ///               with barriers required for interface resources.
    ///
    ///
    fn build(
        &self,
        device: &D,
        aux: &mut T,
        pool: FramePool<D::CommandPool, D::CommandBuffer, <Self::Node as Node<D, T>>::Capability>,
        resources: Resources<D::Buffer, D::Image>,
    ) -> Self::Node;
}

/// Trait-object safe `Node`.
pub trait AnyNode<D: Device + ?Sized, T: ?Sized> {
    /// Record commands required by node.
    /// Recorded buffers go into `submits`.
    fn run(
        &mut self,
        device: &D,
        aux: &T,
        frame: &Frame<D::Fence>,
        raw_submits: &mut Vec<D::Submit>,
    );
}

impl<D, T, N> AnyNode<D, T> for N
where
    D: Device + ?Sized,
    T: ?Sized,
    N: Node<D, T>,
{
    fn run(
        &mut self,
        device: &D,
        aux: &T,
        frame: &Frame<D::Fence>,
        raw_submits: &mut Vec<D::Submit>,
    ) {
        let submits = Node::run(self, device, aux, frame)
            .into_iter()
            .map(|submit| unsafe {
                // Graph guarantee to submit those within frame to the correct queue.
                submit.into_inner().unbind()
            });

        raw_submits.extend(submits);
    }
}

pub trait AnyNodeDesc<D: Device + ?Sized, T: ?Sized> {
    fn build(
        &self,
        device: &D,
        aux: &mut T,
        pool: FramePool<D::CommandPool, D::CommandBuffer, CapabilityFlags>,
        resources: Resources<D::Buffer, D::Image>,
    ) -> Box<dyn AnyNode<D, T>>;
}

impl<D, T, N> AnyNodeDesc<D, T> for N
where
    D: Device + ?Sized,
    T: ?Sized,
    N: NodeDesc<D, T>,
{
    fn build(
        &self,
        device: &D,
        aux: &mut T,
        pool: FramePool<D::CommandPool, D::CommandBuffer, CapabilityFlags>,
        resources: Resources<D::Buffer, D::Image>,
    ) -> Box<dyn AnyNode<D, T>> {
        let node = NodeDesc::build(
            self,
            device,
            aux,
            pool.cast_capability()
                .expect("Must have correct capability"),
            resources,
        );
        Box::new(node)
    }
}

pub struct NodeBuilder<D: Device + ?Sized, T: ?Sized> {
    pub(crate) desc: Box<AnyNodeDesc<D, T>>,
    pub(crate) buffers: Vec<Id>,
    pub(crate) images: Vec<Id>,
    pub(crate) dependencies: Vec<usize>,
}

impl<D, T> NodeBuilder<D, T>
where
    D: Device + ?Sized,
    T: ?Sized,
{
    pub fn new<N>() -> Self
    where
        N: Node<D, T>,
        N::Desc: Default,
    {
        NodeBuilder {
            desc: Box::new(N::desc()),
            buffers: Vec::new(),
            images: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn add_buffer(&mut self, buffer: Id) -> &mut Self {
        self.buffers.push(buffer);
        self
    }

    pub fn add_image(&mut self, image: Id) -> &mut Self {
        self.images.push(image);
        self
    }

    pub fn add_dependency(&mut self, dependency: usize) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    pub fn with_buffer(mut self, buffer: Id) -> Self {
        self.add_buffer(buffer);
        self
    }

    pub fn with_image(mut self, image: Id) -> Self {
        self.add_image(image);
        self
    }

    pub fn with_dependency(mut self, dependency: usize) -> Self {
        self.add_dependency(dependency);
        self
    }
}
