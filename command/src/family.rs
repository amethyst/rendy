//! Family module docs.

use crate::{
    buffer::{Reset, Submittable},
    capability::{Capability, Supports},
    pool::CommandPool,
};

/// Command queue submission.
#[derive(Debug)]
pub struct Submission<W, C, S> {
    /// Iterator over semaphores with stage flag to wait on.
    pub waits: W,

    /// Iterator over semaphores to signal.
    pub signals: S,

    /// Iterator over submittables.
    pub submits: C,
}

/// Family of the command queues.
/// Queues from one family can share resources and execute command buffers associated with the family.
/// All queues of the family have same capabilities.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Family<B: gfx_hal::Backend, C = gfx_hal::QueueType> {
    index: gfx_hal::queue::QueueFamilyId,
    #[derivative(Debug = "ignore")] queues: Vec<B::CommandQueue>,
    // min_image_transfer_granularity: gfx_hal::image::Extent,
    capability: C,
    relevant: relevant::Relevant,
}

impl<B> Family<B>
where
    B: gfx_hal::Backend,
{
    /// Query queue family from device.
    ///
    /// # Safety
    ///
    /// This function shouldn't be used more then once with the same parameters.
    /// Raw queue handle queried from device can make `Family` usage invalid.
    /// `family` must be one of the family indices used during `device` creation.
    /// `properties` must be the properties retuned for queue family from physical device.
    pub unsafe fn from_device(
        queues: &mut gfx_hal::queue::Queues<B>,
        index: gfx_hal::queue::QueueFamilyId,
        count: usize,
        family: &impl gfx_hal::queue::QueueFamily,
    ) -> Self {
        Family {
            index,
            queues: {
                let queues = queues.take_raw(index).expect("");
                assert_eq!(queues.len(), count);
                queues
            },
            // min_image_transfer_granularity: properties.min_image_transfer_granularity,
            capability: family.queue_type(),
            relevant: relevant::Relevant,
        }
    }
}

impl<B, C> Family<B, C>
where
    B: gfx_hal::Backend,
{
    /// Get id of the family.
    pub fn index(&self) -> gfx_hal::queue::QueueFamilyId {
        self.index
    }

    /// Get queues of the family.
    pub fn queues(&self) -> &[impl gfx_hal::queue::RawCommandQueue<B>] {
        &self.queues
    }

    /// Get queues of the family.
    pub fn queues_mut(&mut self) -> &mut [impl gfx_hal::queue::RawCommandQueue<B>] {
        &mut self.queues
    }

    /// Submit commands to the queue of the family.
    pub unsafe fn submit<'a>(&mut self,
        queue: usize,
        submissions: impl IntoIterator<Item = Submission<
            impl IntoIterator<Item = (&'a (impl std::borrow::Borrow<B::Semaphore> + 'a), gfx_hal::pso::PipelineStage)>,
            impl IntoIterator<Item = impl Submittable<B>>,
            impl IntoIterator<Item = &'a (impl std::borrow::Borrow<B::Semaphore> + 'a)>,
        >>,
        fence: Option<&B::Fence>,
    ) {
        let mut submissions = submissions.into_iter().peekable();
        if submissions.peek().is_none() && fence.is_some() {
            gfx_hal::queue::RawCommandQueue::submit(
                &mut self.queues[queue],
                gfx_hal::queue::Submission {
                    command_buffers: std::iter::empty::<&'a B::CommandBuffer>(),
                    wait_semaphores: std::iter::empty::<(&'a B::Semaphore, _)>(),
                    signal_semaphores: std::iter::empty::<&'a B::Semaphore>(),
                },
                fence,
            );
        } else {
            let index = self.index;
            while let Some(submission) = submissions.next() {
                gfx_hal::queue::RawCommandQueue::submit(
                    &mut self.queues[queue],
                    gfx_hal::queue::Submission {
                        command_buffers: submission.submits.into_iter().map(|submit| {
                            assert_eq!(submit.family(), index);
                            unsafe {
                                &*submit.raw()
                            }
                        }),
                        wait_semaphores: submission.waits.into_iter().map(|w| (w.0.borrow(), w.1)),
                        signal_semaphores: submission.signals.into_iter().map(|s| s.borrow()),
                    },
                    submissions.peek().map_or(fence, |_| None),
                );
            }
        }
    }

    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    pub fn create_pool<R>(
        &self,
        device: &impl gfx_hal::Device<B>,
    ) -> Result<CommandPool<B, C, R>, gfx_hal::device::OutOfMemory>
    where
        R: Reset,
        C: Capability,
    {
        let reset = R::default();
        let pool = unsafe {
            // Is this family belong to specified device.
            let raw = device.create_command_pool(
                self.index,
                reset.flags(),
            )?;

            CommandPool::from_raw(raw, self.capability, reset, self.index)
        };

        Ok(pool)
    }

    /// Get family capability.
    pub fn capability(&self) -> C
    where
        C: Capability
    {
        self.capability
    }

    /// Dispose of queue family container.
    pub fn dispose(self) {
        for queue in self.queues {
            gfx_hal::queue::RawCommandQueue::wait_idle(&queue)
                .unwrap();
        }

        self.relevant.dispose();
    }

    /// Convert capability from type-level to value-level.
    pub fn with_queue_type(self) -> Family<B, gfx_hal::QueueType>
    where
        C: Capability,
    {
        Family {
            index: self.index,
            queues: self.queues,
            // min_image_transfer_granularity: self.min_image_transfer_granularity,
            capability: self.capability.into_queue_type(),
            relevant: self.relevant,
        }
    }

    /// Convert capability into type-level one.
    /// 
    pub fn with_capability<U>(self) -> Result<Family<B, U>, Self>
    where
        C: Supports<U>,
    {
        if let Some(capability) = self.capability.supports() {
            Ok(Family {
                index: self.index,
                queues: self.queues,
                // min_image_transfer_granularity: self.min_image_transfer_granularity,
                capability,
                relevant: self.relevant,
            })
        } else {
            Err(self)
        }
    }
}

/// Query queue families from device.
///
/// # Safety
///
/// This function shouldn't be used more then once with same parameters.
/// Raw queue handle queried from device can make returned `Family` usage invalid.
/// `families` iterator must yeild unique family indices with queue count used during `device` creation.
/// `properties` must contain properties retuned for queue family from physical device for each family index yielded by `families`.
pub unsafe fn families_from_device<B>(
    queues: &mut gfx_hal::queue::Queues<B>,
    families: impl IntoIterator<Item = (gfx_hal::queue::QueueFamilyId, usize)>,
    queue_types: &[impl gfx_hal::queue::QueueFamily],
) -> Vec<Family<B>>
where
    B: gfx_hal::Backend,
{
    families
        .into_iter()
        .map(|(index, count)| {
            Family::from_device(queues, index, count, &queue_types[index.0])
        }).collect()
}
