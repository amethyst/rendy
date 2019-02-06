
use {
    super::{QueueId, submission::*},
    crate::{
        buffer::{Reset, Submittable, Submit, NoSimultaneousUse, PrimaryLevel, OutsideRenderPass},
        fence::*,
    },
    gfx_hal::{Backend, Device, queue::RawCommandQueue},
    std::{collections::VecDeque, ops::Range, cmp::max},
};

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Queue<B: Backend> {
    #[derivative(Debug = "ignore")]
    raw: B::CommandQueue,
    id: QueueId,
    epoch: Range<u64>,
}

impl<B> Queue<B>
where
    B: Backend,
{
    pub(super) fn new(raw: B::CommandQueue, id: QueueId) -> Self {
        Queue {
            id,
            raw,
            epoch: 0 .. 0,
        }
    }

    pub fn raw(&mut self) -> &mut impl RawCommandQueue<B> {
        &mut self.raw
    }

    /// Show this queue one of complete fence that was submitted to it.
    /// This bumps latest complete epoch.
    /// Calling this method is crucial for resource cleanup.
    pub fn bump_epoch(&mut self, fence: &CompleteFence<B>) {
        assert_eq!(self.id, fence.queue());
        self.epoch.start = max(self.epoch.start, fence.epoch() + 1);
    }

    /// Returns epoch counter in form `complete .. next`
    /// This means that when you acquire resource that was possibly used used previously
    /// you can check `next` epoch of all queues
    /// and later when `complete` epoch of those queues is equal or greater you can be sure
    /// that resource won't be used by any commands submitted before you acquired it.
    /// 
    /// This counter is used by `resource` system.
    /// It catches any dropped resources and waits for epochs to complete before destroying them.
    pub fn epochs(&self) -> Range<u64> {
        self.epoch.clone()
    }

    /// Submit commands to the queue of the family.
    /// Fence must be armed.
    pub unsafe fn submit<'a>(&mut self,
        submissions: impl IntoIterator<Item = Submission<
            B,
            impl IntoIterator<Item = (&'a (impl std::borrow::Borrow<B::Semaphore> + 'a), gfx_hal::pso::PipelineStage)>,
            impl IntoIterator<Item = impl Submittable<B>>,
            impl IntoIterator<Item = &'a (impl std::borrow::Borrow<B::Semaphore> + 'a)>,
        >>,
        fence: Option<&B::Fence>,
    ) {
        let mut submissions = submissions.into_iter().peekable();
        if submissions.peek().is_none() && fence.is_some() {
            self.raw.submit(
                gfx_hal::queue::Submission {
                    command_buffers: std::iter::empty::<&'a B::CommandBuffer>(),
                    wait_semaphores: std::iter::empty::<(&'a B::Semaphore, _)>(),
                    signal_semaphores: std::iter::empty::<&'a B::Semaphore>(),
                },
                fence
            );
        } else {
            let family = self.id.family();
            while let Some(submission) = submissions.next() {
                self.raw.submit(
                    gfx_hal::queue::Submission {
                        command_buffers: submission.submits.into_iter().map(|submit| {
                            assert_eq!(submit.family(), family);
                            submit.raw()
                        }),
                        wait_semaphores: submission.waits.into_iter().map(|w| (w.0.borrow(), w.1)),
                        signal_semaphores: submission.signals.into_iter().map(|s| s.borrow()),
                    },
                    submissions.peek().map_or(fence, |_| None),
                );
            }
        }
    }

    /// Submit commands to the queue of the family.
    /// Fence must be armed.
    pub unsafe fn submit_with_fence<'a>(&mut self,
        submissions: impl IntoIterator<Item = Submission<
            B,
            impl IntoIterator<Item = (&'a (impl std::borrow::Borrow<B::Semaphore> + 'a), gfx_hal::pso::PipelineStage)>,
            impl IntoIterator<Item = impl Submittable<B>>,
            impl IntoIterator<Item = &'a (impl std::borrow::Borrow<B::Semaphore> + 'a)>,
        >>,
        fence: &mut Fence<B>,
    ) {
        assert!(fence.is_armed());

        let mut submissions = submissions.into_iter().peekable();
        if submissions.peek().is_none() {
            self.raw.submit(
                gfx_hal::queue::Submission {
                    command_buffers: std::iter::empty::<&'a B::CommandBuffer>(),
                    wait_semaphores: std::iter::empty::<(&'a B::Semaphore, _)>(),
                    signal_semaphores: std::iter::empty::<&'a B::Semaphore>(),
                },
                Some(fence.raw()),
            );
        } else {
            let family = self.id.family();
            while let Some(submission) = submissions.next() {
                self.raw.submit(
                    gfx_hal::queue::Submission {
                        command_buffers: submission.submits.into_iter().map(|submit| {
                            assert_eq!(submit.family(), family);
                            submit.raw()
                        }),
                        wait_semaphores: submission.waits.into_iter().map(|w| (w.0.borrow(), w.1)),
                        signal_semaphores: submission.signals.into_iter().map(|s| s.borrow()),
                    },
                    submissions.peek().map_or(Some(fence.raw()), |_| None),
                );
            }
        }

        fence.arm(self.id, self.epoch.end);
        self.epoch.end += 1;
    }

    /// Submit commands to the queue of the family.
    pub unsafe fn submit_with_unarmed<'a>(&mut self,
        submissions: impl IntoIterator<Item = Submission<
            B,
            impl IntoIterator<Item = (&'a (impl std::borrow::Borrow<B::Semaphore> + 'a), gfx_hal::pso::PipelineStage)>,
            impl IntoIterator<Item = impl Submittable<B>>,
            impl IntoIterator<Item = &'a (impl std::borrow::Borrow<B::Semaphore> + 'a)>,
        >>,
        fence: UnarmedFence<B>,
    ) -> ArmedFence<B> {
        let mut submissions = submissions.into_iter().peekable();
        if submissions.peek().is_none() {
            self.raw.submit(
                gfx_hal::queue::Submission {
                    command_buffers: std::iter::empty::<&'a B::CommandBuffer>(),
                    wait_semaphores: std::iter::empty::<(&'a B::Semaphore, _)>(),
                    signal_semaphores: std::iter::empty::<&'a B::Semaphore>(),
                },
                Some(&fence.raw()),
            );
        } else {
            let family = self.id.family();
            while let Some(submission) = submissions.next() {
                self.raw.submit(
                    gfx_hal::queue::Submission {
                        command_buffers: submission.submits.into_iter().map(|submit| {
                            assert_eq!(submit.family(), family);
                            submit.raw()
                        }),
                        wait_semaphores: submission.waits.into_iter().map(|w| (w.0.borrow(), w.1)),
                        signal_semaphores: submission.signals.into_iter().map(|s| s.borrow()),
                    },
                    submissions.peek().map_or(Some(&fence.raw()), |_| None),
                );
            }
        }

        fence.arm(self.id, self.epoch.end - 1)
    }

    /// Submit commands to the queue of the family.
    pub unsafe fn submit_no_fence<'a>(&mut self,
        submissions: impl IntoIterator<Item = Submission<
            B,
            impl IntoIterator<Item = (&'a (impl std::borrow::Borrow<B::Semaphore> + 'a), gfx_hal::pso::PipelineStage)>,
            impl IntoIterator<Item = impl Submittable<B>>,
            impl IntoIterator<Item = &'a (impl std::borrow::Borrow<B::Semaphore> + 'a)>,
        >>,
    ) {
        let mut submissions = submissions.into_iter();
        let family = self.id.family();
        while let Some(submission) = submissions.next() {
            self.raw.submit(
                gfx_hal::queue::Submission {
                    command_buffers: submission.submits.into_iter().map(|submit| {
                        assert_eq!(submit.family(), family);
                        submit.raw()
                    }),
                    wait_semaphores: submission.waits.into_iter().map(|w| (w.0.borrow(), w.1)),
                    signal_semaphores: submission.signals.into_iter().map(|s| s.borrow()),
                },
                None,
            );
        }
    }

    pub fn wait_idle(&self) -> Result<(), gfx_hal::error::HostExecutionError> {
        self.raw.wait_idle()
    }
}
