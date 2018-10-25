// use std::{
//     borrow::Borrow, collections::HashMap, iter::once, marker::PhantomData, ops::AddAssign,
//     sync::{atomic::AtomicUsize, Arc},
// };

use chain;
use command::{Device, FamilyId, Queue, FrameGen, Submission, CapabilityFlags};
use resource::{buffer, image};

// use smallvec::SmallVec;

use node::{
    Node, AnyNode, AnyNodeDesc,
};

// use util::*;

/// Graph that renders whole frame.
#[allow(missing_debug_implementations)]
pub struct Graph<D: Device, T> {
    nodes: Vec<Box<dyn AnyNode<D, T>>>,
    schedule: chain::Schedule<chain::SyncData<usize, usize>>,
    semaphores: Vec<D::Semaphore>,
    buffers: Vec<buffer::Buffer<D::Memory, D::Buffer>>,
    images: Vec<image::Image<D::Memory, D::Image>>,
    frame_gen: FrameGen,
    fences: Vec<D::Fence>,
}

impl<D, T> Graph<D, T>
where
    D: Device,
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
        mut queues: Q,
        device: &mut D,
        aux: &mut T,
    ) -> Vec<D::Fence>
    where
        Q: FnMut(FamilyId, usize) -> Option<&'a mut Queue<D::CommandQueue, CapabilityFlags>>
    {
        unsafe {
            device.reset_fences(&self.fences);
        }
        while self.fences.len() < self.schedule.queue_count() {
            self.fences.push(unsafe { // ?
                device.create_fence(Default::default())
            });
        }

        let frame = self.frame_gen.next_with_fences(::std::mem::replace(&mut self.fences, Vec::new()));

        let ref semaphores = self.semaphores;

        let mut fence_index = 0;
        let mut ready = Submission { waits: Vec::new(), buffers: Vec::new(), signals: Vec::new() };

        for submission in self.schedule.ordered() {
            let sid = submission.id();
            let qid = sid.queue();
            let queue = queues(FamilyId(qid.family().0), qid.index()).expect("`queues` must contain all queues");

            let node = self.nodes.get_mut(submission.node()).expect("`Node` must exist");

            let mut node_submits = Vec::new();

            node.run(
                device,
                aux,
                &frame,
                &mut node_submits,
            );

            let last_in_queue = sid.index() + 1 == self.schedule.queue(qid).unwrap().len();
            let fence = if last_in_queue {
                fence_index += 1;
                Some(&self.fences[fence_index - 1])
            } else {
                None
            };

            if !submission.sync().wait.is_empty() && !ready.buffers.is_empty() {
                // Flush `ready`.
                unimplemented!()
            }

            ready.waits.extend(submission.sync().wait.iter().map(|wait|(&semaphores[*wait.semaphore()], wait.stage())));
            ready.buffers.extend(node_submits);
            ready.signals.extend(submission.sync().signal.iter().map(|signal| &semaphores[*signal.semaphore()]));
            
            if !submission.sync().signal.is_empty() {
                // Flush `ready`.
                unimplemented!()
            }
        }

        // Flush `ready`.
        // Make pending frame.
        unimplemented!()
    }
}

// pub struct GraphBuilder<B: Backend, D, T, U, I> {
//     nodes: Vec<Option<Box<AnyNodeBuilder<B, D, T, U, I>>>>,
//     buffers: Vec<u64>,
//     images: Vec<(image::Kind, Format, Option<ClearValue>)>,
// }

// impl<B, D, T, U, I> GraphBuilder<B, D, T, U, I>
// where
//     B: Backend,
//     D: Device<B>,
//     U: Borrow<B::Buffer>,
//     I: Borrow<B::Image>,
// {
//     /// Create new `GraphBuilder`
//     pub fn new() -> Self {
//         GraphBuilder {
//             nodes: Vec::new(),
//             buffers: Vec::new(),
//             images: Vec::new(),
//         }
//     }

//     /// Create new buffer owned by graph.
//     pub fn create_buffer(&mut self, size: u64) -> BufferId {
//         self.buffers.push(size);
//         BufferId(Id::new(self.buffers.len() as u32 - 1))
//     }

//     /// Create new image owned by graph.
//     pub fn create_image(
//         &mut self,
//         kind: image::Kind,
//         format: Format,
//         clear: Option<ClearValue>,
//     ) -> ImageId {
//         self.images.push((kind, format, clear));
//         ImageId(Id::new(self.images.len() as u32 - 1))
//     }

//     /// Add node to the graph.
//     pub fn add_node<N>(&mut self, builder: NodeBuilder<N>) -> NodeId
//     where
//         N: Node<B, D, T>,
//     {
//         self.nodes.push(Some(Box::new(builder)));
//         NodeId(PassId(self.nodes.len() - 1))
//     }

//     /// Build `Graph`.
//     ///
//     /// # Parameters
//     ///
//     /// `frames`        - maximum number of frames `Graph` will render simultaneously.
//     ///
//     /// `families`      - `Iterator` of `B::QueueFamily`s.
//     ///
//     /// `device`    - `Device<B>` implementation. `B::Device` or wrapper.
//     ///
//     /// `aux`       - auxiliary data that `Node`s use.
//     pub fn build<'a, F, X, Y, P>(
//         self,
//         families: F,
//         mut buffer: X,
//         mut image: Y,
//         presents: P,
//         device: &mut D,
//         aux: &mut T,
//     ) -> Graph<B, D, T, U, I>
//     where
//         F: IntoIterator,
//         F::Item: Borrow<B::QueueFamily>,
//         X: FnMut(u64, buffer::Usage, &mut D, &mut T) -> U,
//         Y: FnMut(image::Kind, Format, image::Usage, &mut D, &mut T) -> I,
//         P: IntoIterator<Item = PresentBuilder<'a, B>>,
//     {
//         trace!("Build Graph");
//         use chain::{build, pass::Pass};

//         let families = families.into_iter().collect::<Vec<_>>();
//         let families = families.iter().map(Borrow::borrow).collect::<Vec<_>>();

//         let mut nodes: Vec<Option<Box<AnyNodeBuilder<B, D, T, U, I> + 'a>>> = self.nodes;
//         let present_dependencies: Vec<_> = (0..nodes.len()).map(PassId).collect();

//         for present in presents {
//             nodes.push(Some(Box::new(
//                 present.with_dependencies(present_dependencies.clone()),
//             )));
//         }

//         trace!("Schedule nodes execution");
//         let passes: Vec<Pass> = nodes
//             .iter()
//             .enumerate()
//             .map(|(i, b)| b.as_ref().unwrap().pass(PassId(i), &families))
//             .collect();

//         let chains = collect(passes, |qid| {
//             find_family::<B, _>(families.iter().cloned(), qid).max_queues()
//         });

//         trace!("Scheduled nodes execution {:#?}", chains);

//         trace!("Allocate buffers");
//         let buffers = self
//             .buffers
//             .iter()
//             .enumerate()
//             .map(|(index, &size)| {
//                 let usage = chains
//                     .buffers
//                     .get(&Id::new(index as u32))
//                     .map_or(buffer::Usage::empty(), |chain| chain.usage());
//                 BufferResource {
//                     size,
//                     buffer: buffer(size, usage, device, aux),
//                 }
//             })
//             .collect::<Vec<_>>();

//         trace!("Allocate images");
//         let images = self
//             .images
//             .iter()
//             .enumerate()
//             .map(|(index, &(kind, format, clear))| {
//                 let usage = chains
//                     .images
//                     .get(&Id::new(index as u32))
//                     .map_or(image::Usage::empty(), |chain| chain.usage());
//                 ImageResource {
//                     kind,
//                     format,
//                     clear,
//                     image: image(kind, format, usage, device, aux),
//                 }
//             })
//             .collect::<Vec<_>>();

//         let mut built_nodes: Vec<Option<Box<AnyNode<B, D, T>>>> =
//             (0..nodes.len()).map(|_| None).collect();

//         trace!("Synchronize");
//         let mut semaphores = GenId::new();
//         let schedule = sync(&chains, || {
//             let id = semaphores.next();
//             (id, id)
//         });
//         trace!("Schedule: {:#?}", schedule);

//         trace!("Build nodes");
//         for family in schedule.iter() {
//             trace!("For family {:#?}", family);
//             for queue in family.iter() {
//                 trace!("For queue {:#?}", queue.id());
//                 for (sid, submission) in queue.iter() {
//                     trace!("For submission {:#?}", sid);
//                     let builder = nodes[submission.pass().0].take().unwrap();
//                     trace!("Build node {}", builder.name());
//                     let node = builder.build(
//                         submission,
//                         &chains.buffers,
//                         &buffers,
//                         &chains.images,
//                         &images,
//                         find_family::<B, _>(families.iter().cloned(), sid.family()),
//                         device,
//                         aux,
//                     );
//                     built_nodes[submission.pass().0] = Some(node);
//                 }
//             }
//         }

//         Graph {
//             nodes: built_nodes.into_iter().map(|node| node.unwrap()).collect(),
//             schedule,
//             semaphores: (0..semaphores.total())
//                 .map(|_| device.create_semaphore())
//                 .collect(),
//             buffers,
//             images,
//         }
//     }
// }

// struct GenId<T> {
//     next: T,
// }

// impl<T> GenId<T>
// where
//     T: Copy + From<u8> + AddAssign,
// {
//     fn new() -> Self {
//         GenId { next: 0.into() }
//     }

//     fn next(&mut self) -> T {
//         let last = self.next;
//         self.next += 1u8.into();
//         last
//     }

//     fn total(self) -> T {
//         self.next
//     }
// }

// fn find_family<'a, B, F>(families: F, qid: QueueFamilyId) -> &'a B::QueueFamily
// where
//     B: Backend,
//     F: IntoIterator<Item = &'a B::QueueFamily>,
// {
//     families.into_iter().find(|qf| qf.id() == qid).unwrap()
// }
