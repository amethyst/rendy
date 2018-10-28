//! Defines node - building block for framegraph.
//!

use std::any::Any;
use ash::{version::DeviceV1_0, vk::{QueueFlags}};

use chain::{
    State,
    BufferState,
    ImageState,
    Id,
};

use command::{
    Encoder,
    Capability,
    FamilyIndex,
    Frames,
};

use resource::{Buffer, Image};

/// The node is building block of the framegraph.
/// Node defines set of resources and operations to perform over them.
/// Read-only data for operations comes from auxiliary data source `T`.
///
/// # Parameters
///
/// `D` - device type.
/// `T` - auxiliary data type.
///
pub trait Node<T: ?Sized>: Sized + Sync + Send + 'static {
    /// Capability required by node.
    /// Graph will execute this node on command queue that supports this capability level.
    type Capability: Capability;

    /// Description type to instantiate the node.
    type Desc: NodeDesc<T, Node = Self>;

    /// Desc creation.
    /// Convenient method if builder implements `Default`.
    fn desc() -> Self::Desc
    where
        Self::Desc: Default,
    {
        Default::default()
    }

    /// Builder creation.
    fn builder(self) -> NodeBuilder<T>
    where
        Self::Desc: Default,
    {
        Self::desc().builder()
    }

    /// Record commands required by node.
    /// Returned submits are guaranteed to be submitted within specified frame.
    fn run<'a, E, F>(
        &mut self,
        device: &impl DeviceV1_0,
        aux: &mut T,
        complete_frame: &'a CompleteFrame,
        frames: &'a Frames,
        encoder: E,
    )
    where
        E: Encoder<Self::Capability>,
    ;
}

/// Resources wrapper.
/// Wraps resources requested by the node.
/// This wrapper guarantees that lifetime of resources is bound to the node lifetime.
#[derive(Clone, Debug)]
pub struct Resources<'a> {
    buffers: Vec<&'a Buffer>,
    images: Vec<&'a Image>,
}

/// Builder of the node.
/// Implementation of the builder type provide framegraph with static information about node
/// that is used for building the node.
pub trait NodeDesc<T: ?Sized>: Sized + 'static {
    /// Node this builder builds.
    type Node: Node<T>;

    /// Builder creation.
    fn builder(self) -> NodeBuilder<T> {
        NodeBuilder::new(self)
    }

    /// Get set or buffer resources the node uses.
    fn buffers(&self) -> Vec<BufferState> {
        Vec::new()
    }

    /// Get set or image resources the node uses.
    fn images(&self) -> Vec<ImageState> {
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
        device: &impl DeviceV1_0,
        aux: &mut T,
        resources: Resources<'_>,
    ) -> Self::Node;
}

/// Trait-object safe `Node`.
pub unsafe trait AnyNode<T: ?Sized>: Any + Sync + Send {
    /// Record commands required by node.
    /// Recorded buffers go into `submits`.
    fn run(
        &mut self,
        device: &impl DeviceV1_0,
        aux: &mut T,
        frame: &Frame<D::Fence>,
        encoder: &mut AnyEncoder<D>,
    );
}

unsafe impl<T, N> AnyNode<T> for N
where
    T: ?Sized,
    N: Node<T>,
{
    fn run(
        &mut self,
        device: &impl DeviceV1_0,
        aux: &mut T,
        frame: &Frame<D::Fence>,
        encoder: &mut AnyEncoder<D>,
    ) {
        Node::run(self, device, aux, frame, encoder.capability::<N::Capability>())
    }
}

/// Trait-object safe `NodeDesc`.
pub unsafe trait AnyNodeDesc<T: ?Sized> {
    /// Find family suitable for the node.
    fn family(&self, families: &[(CapabilityFlags, FamilyIndex)]) -> Option<FamilyIndex>;

    /// Build the node.
    fn build(
        &self,
        device: &impl DeviceV1_0,
        aux: &mut T,
        resources: Resources<'_, D::Buffer, D::Image>,
    ) -> Box<dyn AnyNode<T>>;
}

unsafe impl<T, N> AnyNodeDesc<T> for N
where
    T: ?Sized,
    N: NodeDesc<T>,
{
    fn family(&self, families: &[(CapabilityFlags, FamilyIndex)]) -> Option<FamilyIndex> {
        families
            .iter()
            .find(|&(cap, _)| <N::Node as Node<T>>::Capability::from_flags(*cap).is_some())
            .map(|&(_, id)| id)
    }

    fn build(
        &self,
        device: &impl DeviceV1_0,
        aux: &mut T,
        pool: FramePool<D::CommandPool, D::CommandBuffer, CapabilityFlags>,
        resources: Resources<'_, D::Buffer, D::Image>,
    ) -> Box<dyn AnyNode<T>> {
        let node = NodeDesc::build(
            self,
            device,
            aux,
            pool.cast_capability()
                .map_err(|_| ())
                .expect("Must have correct capability"),
            resources,
        );
        Box::new(node)
    }
}

/// Builder for the node.
#[allow(missing_debug_implementations)]
pub struct NodeBuilder<T: ?Sized> {
    pub(crate) desc: Box<dyn AnyNodeDesc<T>>,
    pub(crate) buffers: Vec<Id>,
    pub(crate) images: Vec<Id>,
    pub(crate) dependencies: Vec<usize>,
}

impl<T> NodeBuilder<T>
where
    D: Device + ?Sized,
    T: ?Sized,
{
    /// Create new builder.
    pub fn new<N>(desc: N) -> Self
    where
        N: NodeDesc<T>,
    {
        NodeBuilder {
            desc: Box::new(desc),
            buffers: Vec::new(),
            images: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// Add buffer to the node.
    /// This method must be called for each buffer node uses.
    pub fn add_buffer(&mut self, buffer: Id) -> &mut Self {
        self.buffers.push(buffer);
        self
    }

    /// Add image to the node.
    /// This method must be called for each image node uses.
    pub fn add_image(&mut self, image: Id) -> &mut Self {
        self.images.push(image);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn add_dependency(&mut self, dependency: usize) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add buffer to the node.
    /// This method must be called for each buffer node uses.
    pub fn with_buffer(mut self, buffer: Id) -> Self {
        self.add_buffer(buffer);
        self
    }

    /// Add image to the node.
    /// This method must be called for each image node uses.
    pub fn with_image(mut self, image: Id) -> Self {
        self.add_image(image);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: usize) -> Self {
        self.add_dependency(dependency);
        self
    }

    /// Build node from this.
    #[allow(unused)]
    pub(crate) fn build(
        &self,
        device: &impl DeviceV1_0,
        aux: &mut T,
        resources: Resources<'_, Buffer, Image>,
    ) -> Box<dyn AnyNode<T>> {
        self.desc.build(device, aux, resources)
    }
}

pub struct AnyEncoder {
    
}
