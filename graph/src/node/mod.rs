//! Defines node - building block for framegraph.
//!

mod render;

use crate::{
    chain,
    command::{Capability, Family, Submit, Supports},
    factory::Factory,
    frame::Frames,
    resource::{Buffer, Image},
};

/// Barrier required for node.
///
/// This type is similar to [`gfx_hal::memory::Barrier`]
/// except that it has resource indices instead of references.
///
/// [`gfx_hal::memory::Barrier`]: ../gfx_hal/memory/enum.Barrier.html
#[derive(Clone, Debug)]
pub enum Barrier {
    /// Applies the given access flags to all buffers in the range.
    AllBuffers(std::ops::Range<gfx_hal::buffer::Access>),
    /// Applies the given access flags to all images in the range.
    AllImages(std::ops::Range<gfx_hal::image::Access>),
    /// A memory barrier that defines access to a buffer.
    Buffer {
        /// The access flags controlling the buffer.
        states: std::ops::Range<gfx_hal::buffer::State>,
        /// The buffer the barrier controls.
        target: usize,
    },
    /// A memory barrier that defines access to (a subset of) an image.
    Image {
        /// The access flags controlling the image.
        states: std::ops::Range<gfx_hal::image::State>,
        /// The image the barrier controls.
        target: usize,
        /// A `SubresourceRange` that defines which section of an image the barrier applies to.
        range: gfx_hal::image::SubresourceRange,
    },
}

/// Buffer shared between nodes.
#[derive(Clone, Copy, Debug)]
pub struct NodeBuffer<'a, B: gfx_hal::Backend> {
    /// Buffer reference.
    pub buffer: &'a Buffer<B>,

    /// Buffer state for node.
    pub state: chain::BufferState,
}

/// Image shared between nodes.
#[derive(Clone, Copy, Debug)]
pub struct NodeImage<'a, B: gfx_hal::Backend> {
    /// Image reference.
    pub image: &'a Image<B>,

    /// Image state for node.
    pub state: chain::ImageState,

    /// Specify that node should clear image to this value.
    pub clear: Option<gfx_hal::command::ClearValue>,
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
    std::fmt::Debug + Sized + Sync + Send + 'static
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
    fn builder(self) -> NodeBuilder<B, T>
    where
        Self::Desc: Default,
    {
        Self::desc().builder()
    }

    /// Record commands required by node.
    /// Returned submits are guaranteed to be submitted within specified frame.
    fn run<'a>(
        &mut self,
        factory: &mut Factory<B>,
        aux: &mut T,
        frames: &'a Frames<B>,
    ) -> Submit<'a, B>;

    /// Dispose of the node.
    fn dispose(self, factory: &mut Factory<B>, aux: &mut T);
}

/// Builder of the node.
/// Implementation of the builder type provide framegraph with static information about node
/// that is used for building the node.
pub trait NodeDesc<B: gfx_hal::Backend, T: ?Sized>: std::fmt::Debug + Sized + 'static {
    /// Node this builder builds.
    type Node: Node<B, T>;

    /// Builder creation.
    fn builder(self) -> NodeBuilder<B, T> {
        NodeBuilder::new(Box::new(self))
    }

    /// Get set or buffer resources the node uses.
    fn buffers(&self) -> Vec<chain::BufferState> {
        Vec::new()
    }

    /// Get set or image resources the node uses.
    fn images(&self) -> Vec<chain::ImageState> {
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
        buffers: impl IntoIterator<Item = NodeBuffer<'a, B>>,
        images: impl IntoIterator<Item = NodeImage<'a, B>>,
        family: gfx_hal::queue::QueueFamilyId,
    ) -> Result<Self::Node, failure::Error>;
}

/// Trait-object safe `Node`.
pub unsafe trait AnyNode<B: gfx_hal::Backend, T: ?Sized>:
    std::fmt::Debug + Sync + Send
{
    /// Record commands required by node.
    /// Recorded buffers go into `submits`.
    fn run<'a>(&mut self, factory: &mut Factory<B>, aux: &mut T, frames: &'a Frames<B>) -> Submit<'a, B>;
}

unsafe impl<B, T, N> AnyNode<B, T> for N
where
    B: gfx_hal::Backend,
    T: ?Sized,
    N: Node<B, T>,
{
    fn run<'a>(&mut self, factory: &mut Factory<B>, aux: &mut T, frames: &'a Frames<B>) -> Submit<'a, B> {
        Node::run(self, factory, aux, frames)
    }
}

/// Trait-object safe `NodeDesc`.
pub unsafe trait AnyNodeDesc<B: gfx_hal::Backend, T: ?Sized>: std::fmt::Debug {
    /// Find family suitable for the node.
    fn family(&self, families: &[Family<B>]) -> Option<gfx_hal::queue::QueueFamilyId>;

    /// Get buffer resource states.
    fn buffers(&self) -> Vec<chain::BufferState>;

    /// Get image resource states.
    fn images(&self) -> Vec<chain::ImageState>;

    /// Build the node.
    fn build<'a>(
        &self,
        factory: &mut Factory<B>,
        aux: &mut T,
        buffers: &[NodeBuffer<'a, B>],
        images: &[NodeImage<'a, B>],
        family: gfx_hal::queue::QueueFamilyId,
    ) -> Result<Box<dyn AnyNode<B, T>>, failure::Error>;
}

unsafe impl<B, T, N> AnyNodeDesc<B, T> for N
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

    fn buffers(&self) -> Vec<chain::BufferState> {
        N::buffers(self)
    }

    fn images(&self) -> Vec<chain::ImageState> {
        N::images(self)
    }

    fn build<'a>(
        &self,
        factory: &mut Factory<B>,
        aux: &mut T,
        buffers: &[NodeBuffer<'a, B>],
        images: &[NodeImage<'a, B>],
        family: gfx_hal::queue::QueueFamilyId,
    ) -> Result<Box<dyn AnyNode<B, T>>, failure::Error> {
        let node = NodeDesc::build(
            self,
            factory,
            aux,
            buffers.iter().cloned(),
            images.iter().cloned(),
            family,
        )?;
        Ok(Box::new(node))
    }
}

/// Builder for the node.
#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
pub struct NodeBuilder<B: gfx_hal::Backend, T: ?Sized> {
    pub(crate) desc: Box<dyn AnyNodeDesc<B, T>>,
    pub(crate) buffers: Vec<chain::Id>,
    pub(crate) images: Vec<chain::Id>,
    pub(crate) dependencies: Vec<usize>,
}

impl<B, T> NodeBuilder<B, T>
where
    B: gfx_hal::Backend,
    T: ?Sized,
{
    /// Create new builder.
    pub fn new(desc: Box<dyn AnyNodeDesc<B, T>>) -> Self {
        NodeBuilder {
            desc,
            buffers: Vec::new(),
            images: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// Add buffer to the node.
    /// This method must be called for each buffer node uses.
    pub fn add_buffer(&mut self, buffer: chain::Id) -> &mut Self {
        self.buffers.push(buffer);
        self
    }

    /// Add image to the node.
    /// This method must be called for each image node uses.
    pub fn add_image(&mut self, image: chain::Id) -> &mut Self {
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
    pub fn with_buffer(mut self, buffer: chain::Id) -> Self {
        self.add_buffer(buffer);
        self
    }

    /// Add image to the node.
    /// This method must be called for each image node uses.
    pub fn with_image(mut self, image: chain::Id) -> Self {
        self.add_image(image);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: usize) -> Self {
        self.add_dependency(dependency);
        self
    }

    pub(crate) fn chain(&self, id: usize, factory: &Factory<B>) -> chain::Node {
        chain::Node {
            id,
            family: self.desc.family(factory.families()).unwrap(),
            dependencies: self.dependencies.clone(),
            buffers: self
                .buffers
                .iter()
                .cloned()
                .zip(self.desc.buffers())
                .collect(),
            images: self
                .images
                .iter()
                .cloned()
                .zip(self.desc.images())
                .collect(),
        }
    }

    /// Build node from this.
    #[allow(unused)]
    pub(crate) fn build<'a>(
        &self,
        factory: &mut Factory<B>,
        aux: &mut T,
        buffers: &[NodeBuffer<'a, B>],
        images: &[NodeImage<'a, B>],
        family: gfx_hal::queue::QueueFamilyId,
    ) -> Result<Box<dyn AnyNode<B, T>>, failure::Error> {
        self.desc.build(factory, aux, buffers, images, family)
    }
}
