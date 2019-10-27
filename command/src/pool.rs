//! CommandPool module docs.

use {
    crate::{buffer::*, capability::*, core::Device, family::FamilyId},
    rendy_core::hal::{device::Device as _, pool::CommandPool as _, Backend},
};

/// Simple pool wrapper.
/// Doesn't provide any guarantees.
/// Wraps raw buffers into `CommandCommand buffer`.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct CommandPool<B: Backend, C = QueueType, R = NoIndividualReset> {
    #[derivative(Debug = "ignore")]
    raw: B::CommandPool,
    capability: C,
    reset: R,
    family: FamilyId,
    relevant: relevant::Relevant,
}

family_owned!(CommandPool<B, C, R>);

impl<B, C, R> CommandPool<B, C, R>
where
    B: Backend,
    R: Reset,
{
    /// Create command pool associated with the family.
    /// Command buffers created from the pool could be submitted to the queues of the family.
    ///
    /// # Safety
    ///
    /// Family must belong to specified device.
    /// Family must have specified capability.
    pub unsafe fn create(
        family: FamilyId,
        capability: C,
        device: &Device<B>,
    ) -> Result<Self, rendy_core::hal::device::OutOfMemory>
    where
        R: Reset,
        C: Capability,
    {
        let reset = R::default();
        let raw = device.create_command_pool(
            rendy_core::hal::queue::QueueFamilyId(family.index),
            reset.flags(),
        )?;
        Ok(CommandPool::from_raw(raw, capability, reset, family))
    }

    /// Wrap raw command pool.
    ///
    /// # Safety
    ///
    /// * `raw` must be valid command pool handle.
    /// * The command pool must be created for specified `family` index.
    /// * `capability` must be subset of capabilites of the `family` the pool was created for.
    /// * if `reset` is `IndividualReset` the pool must be created with individual command buffer reset flag set.
    pub unsafe fn from_raw(raw: B::CommandPool, capability: C, reset: R, family: FamilyId) -> Self {
        CommandPool {
            raw,
            capability,
            reset,
            family,
            relevant: relevant::Relevant,
        }
    }

    /// Allocate new command buffers.
    pub fn allocate_buffers<L: Level>(
        &mut self,
        count: usize,
    ) -> Vec<CommandBuffer<B, C, InitialState, L, R>>
    where
        L: Level,
        C: Capability,
    {
        let level = L::default();

        let buffers = unsafe { self.raw.allocate_vec(count, level.raw_level()) };

        buffers
            .into_iter()
            .map(|raw| unsafe {
                CommandBuffer::from_raw(
                    raw,
                    self.capability,
                    InitialState,
                    level,
                    self.reset,
                    self.family,
                )
            })
            .collect()
    }

    /// Free buffers.
    /// Buffers must be in droppable state.
    /// TODO: Validate buffers were allocated from this pool.
    pub unsafe fn free_buffers(
        &mut self,
        buffers: impl IntoIterator<Item = CommandBuffer<B, C, impl Resettable, impl Level, R>>,
    ) {
        let buffers = buffers
            .into_iter()
            .map(|buffer| buffer.into_raw())
            .collect::<Vec<_>>();

        self.raw.free(buffers);
    }

    /// Reset all buffers of this pool.
    ///
    /// # Safety
    ///
    /// All buffers allocated from this pool must be marked reset.
    /// See [`CommandBuffer::mark_reset`](struct.Command buffer.html#method.mark_reset)
    pub unsafe fn reset(&mut self) {
        rendy_core::hal::pool::CommandPool::reset(&mut self.raw, false);
    }

    /// Dispose of command pool.
    ///
    /// # Safety
    ///
    /// All buffers allocated from this pool must be [freed](#method.free_buffers).
    pub unsafe fn dispose(self, device: &Device<B>) {
        self.assert_device_owner(device);
        device.destroy_command_pool(self.raw);
        self.relevant.dispose();
    }

    /// Convert capability level
    pub fn with_queue_type(self) -> CommandPool<B, QueueType, R>
    where
        C: Capability,
    {
        CommandPool {
            raw: self.raw,
            capability: self.capability.into_queue_type(),
            reset: self.reset,
            family: self.family,
            relevant: self.relevant,
        }
    }

    /// Convert capability level
    pub fn with_capability<U>(self) -> Result<CommandPool<B, U, R>, Self>
    where
        C: Supports<U>,
    {
        if let Some(capability) = self.capability.supports() {
            Ok(CommandPool {
                raw: self.raw,
                capability,
                reset: self.reset,
                family: self.family,
                relevant: self.relevant,
            })
        } else {
            Err(self)
        }
    }
}
