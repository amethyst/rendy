use std::collections::VecDeque;

use hal::device::Device as _;
use rendy_core::hal;
use rendy_factory::Factory;

pub struct Frames<B: hal::Backend> {
    queue_family: hal::queue::QueueFamilyId,

    frames: VecDeque<Frame<B>>,
    pending: Vec<Frame<B>>,
    current: Frame<B>,

    free_semaphores: Vec<B::Semaphore>,
    free_events: Vec<B::Event>,
    free_fences: Vec<B::Fence>,
    free_command_pools: Vec<B::CommandPool>,
}

impl<B: hal::Backend> Frames<B> {
    pub fn new(queue_family: hal::queue::QueueFamilyId) -> Self {
        Frames {
            queue_family,

            frames: VecDeque::new(),
            pending: Vec::new(),
            current: Frame::new(),

            free_semaphores: Vec::new(),
            free_events: Vec::new(),
            free_fences: Vec::new(),
            free_command_pools: Vec::new(),
        }
    }

    pub fn queue_family(&self) -> hal::queue::QueueFamilyId {
        self.queue_family
    }

    /// The given fence needs to finish in order for the frame to be finished.
    pub fn wait_fence(&mut self, fence: B::Fence) {
        self.current.wait_fences.push((fence, false));
    }

    /// Advance to render next frame.
    pub fn advance(&mut self, factory: &Factory<B>) {
        assert!(
            self.current.wait_fences.len() > 0,
            "attempted to advance frame with no work"
        );

        while let Some(mut frame) = self.frames.pop_front() {
            let mut all_finished = true;
            for (fence, finished) in frame.wait_fences.iter_mut() {
                if *finished {
                    continue;
                }

                let status = unsafe { factory.device().get_fence_status(fence) };

                if status.unwrap() {
                    *finished = true;
                } else {
                    all_finished = false;
                }
            }

            if all_finished {
                for semaphore in frame.semaphores.drain(..) {
                    self.free_semaphores.push(semaphore);
                }

                for mut event in frame.events.drain(..) {
                    unsafe {
                        factory.device().reset_event(&mut event).unwrap();
                    }
                    self.free_events.push(event);
                }

                for mut fence in frame
                    .fences
                    .drain(..)
                    .chain(frame.wait_fences.drain(..).map(|(f, _fin)| f))
                {
                    unsafe {
                        factory.device().reset_fence(&mut fence).unwrap();
                    }
                    self.free_fences.push(fence);
                }

                for (mut command_pool, mut buffers) in frame.command_pools.drain(..) {
                    use hal::pool::CommandPool as _;
                    unsafe {
                        command_pool.free(buffers.drain(..));
                        command_pool.reset(true);
                    }

                    self.free_command_pools.push(command_pool);
                }

                self.pending.push(frame);
            } else {
                self.frames.push_front(frame);
            }
        }

        let mut last = self.pending.pop().unwrap_or_else(|| Frame::new());
        std::mem::swap(&mut last, &mut self.current);
        self.frames.push_back(last);
    }

    pub fn get_semaphore(&mut self, factory: &Factory<B>) -> B::Semaphore {
        self.free_semaphores
            .pop()
            .unwrap_or_else(|| factory.device().create_semaphore().unwrap())
    }

    pub fn get_event(&mut self, factory: &Factory<B>) -> B::Event {
        self.free_events
            .pop()
            .unwrap_or_else(|| factory.device().create_event().unwrap())
    }

    pub fn get_fence(&mut self, factory: &Factory<B>) -> B::Fence {
        self.free_fences
            .pop()
            .unwrap_or_else(|| factory.device().create_fence(false).unwrap())
    }

    pub fn get_command_pool(&mut self, factory: &Factory<B>) -> B::CommandPool {
        self.free_command_pools.pop().unwrap_or_else(|| {
            unsafe {
                factory
                    .device()
                    .create_command_pool(
                        self.queue_family,
                        hal::pool::CommandPoolCreateFlags::empty(),
                    )
                    .unwrap()
            }
        })
    }

    pub fn current(&mut self) -> &mut Frame<B> {
        &mut self.current
    }
}

pub struct Frame<B: hal::Backend> {
    pub(crate) semaphores: Vec<B::Semaphore>,
    pub(crate) events: Vec<B::Event>,
    pub(crate) fences: Vec<B::Fence>,
    pub(crate) command_pools: Vec<(B::CommandPool, Vec<B::CommandBuffer>)>,

    wait_fences: Vec<(B::Fence, bool)>,
}

impl<B: hal::Backend> Frame<B> {
    pub fn new() -> Self {
        Frame {
            semaphores: Vec::new(),
            events: Vec::new(),
            fences: Vec::new(),
            command_pools: Vec::new(),

            wait_fences: Vec::new(),
        }
    }
}
