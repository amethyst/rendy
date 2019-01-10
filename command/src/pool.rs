//! CommandPool module docs.

use crate::{buffer::*, capability::*, family::FamilyId};

/// Simple pool wrapper.
/// Doesn't provide any guarantees.
/// Wraps raw buffers into `CommandCommand buffer`.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct CommandPool<B: gfx_hal::Backend, C = gfx_hal::QueueType, R = NoIndividualReset> {
    #[derivative(Debug = "ignore")]raw: B::CommandPool,
    capability: C,
    reset: R,
    family: FamilyId,
    relevant: relevant::Relevant,
}

impl<B, C, R> CommandPool<B, C, R>
where
    B: gfx_hal::Backend,
    R: Reset,
{
    /// Wrap raw command pool.
    ///
    /// # Safety
    ///
    /// * `raw` must be valid command pool handle.
    /// * The command pool must be created for specified `family` index.
    /// * `capability` must be subset of capabilites of the `family` the pool was created for.
    /// * if `reset` is `IndividualReset` the pool must be created with individual command buffer reset flag set.
    pub unsafe fn from_raw(
        raw: B::CommandPool,
        capability: C,
        reset: R,
        family: FamilyId,
    ) -> Self {
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

        let buffers = gfx_hal::pool::RawCommandPool::allocate_vec(
            &mut self.raw,
            count,
            level.raw_level(),
        );

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
            }).collect()
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
            .map(|buffer| unsafe { buffer.into_raw() })
            .collect::<Vec<_>>();

        unsafe {
            gfx_hal::pool::RawCommandPool::free(&mut self.raw, buffers);
        }
    }

    /// Reset all buffers of this pool.
    ///
    /// # Safety
    ///
    /// All buffers allocated from this pool must be marked reset.
    /// See [`CommandBuffer::mark_reset`](struct.Command buffer.html#method.mark_reset)
    pub unsafe fn reset(&mut self) {
        gfx_hal::pool::RawCommandPool::reset(&mut self.raw);
    }

    /// Dispose of command pool.
    ///
    /// # Safety
    ///
    /// * All buffers allocated from this pool must be [freed](#method.free_buffers).
    pub unsafe fn dispose(self, device: &impl gfx_hal::Device<B>) {
        device.destroy_command_pool(self.raw);
        self.relevant.dispose();
    }

    /// Convert capability level
    pub fn with_queue_type(self) -> CommandPool<B, gfx_hal::QueueType, R>
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
