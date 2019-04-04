use {
    super::{submission::*, QueueId},
    crate::{buffer::Submittable, fence::*},
    gfx_hal::{queue::RawCommandQueue, Backend},
};

/// Command queue wrapper.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Queue<B: Backend> {
    #[derivative(Debug = "ignore")]
    raw: B::CommandQueue,
    id: QueueId,
    next_epoch: u64,
}

family_owned!(@NOCAP Queue<B> @ |q: &Self| q.id.family);

impl<B> Queue<B>
where
    B: Backend,
{
    pub(super) fn new(raw: B::CommandQueue, id: QueueId) -> Self {
        Queue {
            id,
            raw,
            next_epoch: 0,
        }
    }

    /// Id of the queue.
    pub fn id(&self) -> QueueId {
        self.id
    }

    /// Get raw command queue.
    pub fn raw(&mut self) -> &mut impl RawCommandQueue<B> {
        &mut self.raw
    }

    /// Returns next queue epoch.
    pub fn next_epoch(&self) -> u64 {
        self.next_epoch
    }

    /// Submit commands to the queue of the family.
    /// Fence must be submitted.
    pub unsafe fn submit<'a>(
        &mut self,
        submissions: impl IntoIterator<
            Item = Submission<
                B,
                impl IntoIterator<
                    Item = (
                        &'a (impl std::borrow::Borrow<B::Semaphore> + 'a),
                        gfx_hal::pso::PipelineStage,
                    ),
                >,
                impl IntoIterator<Item = impl Submittable<B>>,
                impl IntoIterator<Item = &'a (impl std::borrow::Borrow<B::Semaphore> + 'a)>,
            >,
        >,
        fence: Option<&mut Fence<B>>,
    ) {
        assert!(fence.as_ref().map_or(true, |f| f.is_unsignaled()));

        let mut submissions = submissions.into_iter().peekable();
        if submissions.peek().is_none() && fence.is_some() {
            self.raw.submit(
                gfx_hal::queue::Submission {
                    command_buffers: std::iter::empty::<&'a B::CommandBuffer>(),
                    wait_semaphores: std::iter::empty::<(&'a B::Semaphore, _)>(),
                    signal_semaphores: std::iter::empty::<&'a B::Semaphore>(),
                },
                fence.as_ref().map(|f| f.raw()),
            );
        } else {
            let family = self.id.family;
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
                    submissions
                        .peek()
                        .map_or(fence.as_ref().map(|f| f.raw()), |_| None),
                );
            }
        }

        if let Some(fence) = fence {
            fence.mark_submitted(FenceEpoch {
                queue: self.id,
                epoch: self.next_epoch,
            });
            self.next_epoch += 1;
        }
    }

    /// Submit commands to the queue of the family.
    /// Fence must be submitted.
    /// This version uses raw fence and doesn't increment epoch.
    pub unsafe fn submit_raw_fence<'a>(
        &mut self,
        submissions: impl IntoIterator<
            Item = Submission<
                B,
                impl IntoIterator<
                    Item = (
                        &'a (impl std::borrow::Borrow<B::Semaphore> + 'a),
                        gfx_hal::pso::PipelineStage,
                    ),
                >,
                impl IntoIterator<Item = impl Submittable<B>>,
                impl IntoIterator<Item = &'a (impl std::borrow::Borrow<B::Semaphore> + 'a)>,
            >,
        >,
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
                fence,
            );
        } else {
            let family = self.id.family;
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

    /// Wait for queue to finish all pending commands.
    pub fn wait_idle(&self) -> Result<(), gfx_hal::error::HostExecutionError> {
        self.raw.wait_idle()
    }
}
