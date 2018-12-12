//! Command buffer module docs.

mod level;
mod reset;
mod state;
mod submit;
mod usage;
mod recording;

use crate::{
    capability::{Capability, Supports},
};

pub use self::{
    level::*,
    reset::*,
    state::*,
    submit::*,
    usage::*,
    recording::*,
};

/// Command buffer wrapper.
/// This wrapper defines state with usage, level and ability to be individually reset at type level.
/// This way many methods become safe.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct CommandBuffer<B: gfx_hal::Backend, C, S, L = PrimaryLevel, R = NoIndividualReset> {
    #[derivative(Debug = "ignore")]
    raw: B::CommandBuffer,
    capability: C,
    state: S,
    level: L,
    reset: R,
    family: gfx_hal::queue::QueueFamilyId,
}

impl<B, C, S, L, R> CommandBuffer<B, C, S, L, R>
where
    B: gfx_hal::Backend,
{
    /// Wrap raw buffer handle.
    ///
    /// # Safety
    ///
    /// * `raw` must be valid command buffer handle.
    /// * `capability` must be subset of `family` capability.
    /// * `state` must represent actual state buffer currently in.
    /// * command buffer must be allocated with specified `level`.
    /// * If `reset` is `IndividualReset` then buffer must be allocated from pool created with `IndividualReset` marker.
    /// * command buffer must be allocated from pool created for `family`.
    pub(crate) unsafe fn from_raw(
        raw: impl Into<B::CommandBuffer>,
        capability: C,
        state: S,
        level: L,
        reset: R,
        family: gfx_hal::queue::QueueFamilyId,
    ) -> Self {
        CommandBuffer {
            raw: raw.into(),
            capability,
            state,
            level,
            reset,
            family,
        }
    }

    /// Get raw command buffer handle.
    ///
    /// # Safety
    ///
    /// * Valid usage for command buffer must not be violated.
    /// Particularly command buffer must not change its state.
    /// Or `change_state` must be used to reflect accumulated change.
    pub unsafe fn raw(&mut self) -> &mut B::CommandBuffer {
        &mut self.raw
    }

    /// Get raw command buffer handle.
    ///
    /// # Safety
    ///
    /// * Valid usage for command buffer must not be violated.
    pub unsafe fn into_raw(self) -> B::CommandBuffer {
        self.raw
    }

    /// Change state of the command buffer.
    ///
    /// # Safety
    ///
    /// * This method must be used only to reflect state changed due to raw handle usage.
    pub unsafe fn change_state<U>(self, f: impl FnOnce(S) -> U) -> CommandBuffer<B, C, U, L, R> {
        CommandBuffer {
            raw: self.raw,
            capability: self.capability,
            state: f(self.state),
            level: self.level,
            reset: self.reset,
            family: self.family,
        }
    }

    /// Get buffers capability.
    pub fn capability(&self) -> C
    where
        C: Capability,
    {
        self.capability
    }

    /// Get buffers family.
    pub fn family(&self) -> gfx_hal::queue::QueueFamilyId {
        self.family
    }

    /// Convert capability level.
    pub fn with_queue_type(self) -> CommandBuffer<B, gfx_hal::QueueType, S, L, R>
    where
        C: Capability,
    {
        CommandBuffer {
            raw: self.raw,
            capability: self.capability.into_queue_type(),
            state: self.state,
            level: self.level,
            reset: self.reset,
            family: self.family,
        }
    }

    /// Convert capability level.
    pub fn with_capability<U>(self) -> Result<CommandBuffer<B, U, S, L, R>, Self>
    where
        C: Supports<U>,
    {
        if let Some(capability) = self.capability.supports() {
            Ok(CommandBuffer {
                raw: self.raw,
                capability: capability,
                state: self.state,
                level: self.level,
                reset: self.reset,
                family: self.family,
            })
        } else {
            Err(self)
        }
    }
}

impl<B, C, L, R> CommandBuffer<B, C, InitialState, L, R>
where
    B: gfx_hal::Backend,
{
    /// Begin recording command buffer.
    ///
    /// # Parameters
    ///
    /// `usage` - specifies usage of the command buffer. Possible types are `OneShot`, `MultiShot`.
    pub fn begin<U, P>(
        mut self,
        usage: U,
        pass_continue: P,
    ) -> CommandBuffer<B, C, RecordingState<U, P>, L, R>
    where
        U: Usage,
        P: Usage,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::begin(
                &mut self.raw,
                usage.flags() | pass_continue.flags(),
                gfx_hal::command::CommandBufferInheritanceInfo::default(),
            );

            self.change_state(|_| RecordingState(usage, pass_continue))
        }
    }
}

impl<B, C, N, L, R> CommandBuffer<B, C, PendingState<N>, L, R>
where
    B: gfx_hal::Backend,
{
    /// Mark command buffer as complete.
    ///
    /// # Safety
    ///
    /// * Commands recoreded to this buffer must be complete.
    /// Normally command buffer moved to this state when [`Submit`] object is created.
    /// To ensure that recorded commands are complete once can [wait] for the [`Fence`] specified
    /// when [submitting] created [`Submit`] object or in later submission to the same queue.
    ///
    /// [`Submit`]: struct.Submit
    /// [wait]: ..gfx_hal/device/trait.Device.html#method.wait_for_fences
    /// [`Fence`]: ..gfx_hal/trait.Backend.html#associatedtype.Fence
    /// [submitting]: ..gfx_hal/queue/struct.CommandQueue.html#method.submit
    pub unsafe fn complete(self) -> CommandBuffer<B, C, N, L, R> {
        self.change_state(|PendingState(state)| state)
    }
}

impl<B, C, S, L> CommandBuffer<B, C, S, L, IndividualReset>
where
    B: gfx_hal::Backend,
    S: Resettable,
{
    /// Reset command buffer.
    pub fn reset(self) -> CommandBuffer<B, C, InitialState, L, IndividualReset> {
        unsafe { self.change_state(|_| InitialState) }
    }
}

impl<B, C, S, L> CommandBuffer<B, C, S, L>
where
    B: gfx_hal::Backend,
    S: Resettable,
{
    /// Mark command buffer as reset.
    ///
    /// # Safety
    ///
    /// * This function must be used only to reflect command buffer being reset implicitly.
    /// For instance:
    /// * [`CommandPool::reset`](struct.CommandPool.html#method.reset) on pool from which the command buffer was allocated.
    /// * Raw handle usage.
    pub unsafe fn mark_reset(self) -> CommandBuffer<B, C, InitialState, L> {
        self.change_state(|_| InitialState)
    }
}
