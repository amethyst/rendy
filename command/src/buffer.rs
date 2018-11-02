//! Command buffer module docs.

use ash::{version::DeviceV1_0, vk};
use relevant::Relevant;
use std::borrow::Borrow;

use crate::{capability::Capability, family::FamilyIndex};

/// Command buffers of this level can be submitted to the command queues.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrimaryLevel;

/// Command buffers of this level can be executed as part of the primary buffers.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecondaryLevel;

/// Command buffer level.
pub trait Level: Copy {
    /// Get raw level value.
    fn level(&self) -> vk::CommandBufferLevel;
}

impl Level for PrimaryLevel {
    fn level(&self) -> vk::CommandBufferLevel {
        vk::CommandBufferLevel::PRIMARY
    }
}

impl Level for SecondaryLevel {
    fn level(&self) -> vk::CommandBufferLevel {
        vk::CommandBufferLevel::SECONDARY
    }
}

impl Level for vk::CommandBufferLevel {
    fn level(&self) -> vk::CommandBufferLevel {
        *self
    }
}

/// This flag specify that buffer can be reset individually.
#[derive(Clone, Copy, Debug, Default)]
pub struct IndividualReset;

/// This flag specify that buffer cannot be reset individually.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoIndividualReset;

/// Specify flags required for command pool creation to allow individual buffer reset.
pub trait Reset: Copy {
    fn flags(&self) -> vk::CommandPoolCreateFlags;
}

impl Reset for IndividualReset {
    fn flags(&self) -> vk::CommandPoolCreateFlags {
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
    }
}

impl Reset for NoIndividualReset {
    fn flags(&self) -> vk::CommandPoolCreateFlags {
        vk::CommandPoolCreateFlags::empty()
    }
}

/// Command buffer state in which all buffers start.
/// Resetting also moves buffer to this state.
#[derive(Clone, Copy, Debug, Default)]
pub struct InitialState;

/// Command buffer in recording state could be populated with commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct RecordingState<U>(U);

/// Command buffer in executable state can be submitted.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExecutableState<U>(U);

/// Command buffer in pending state are submitted to the device.
/// Command buffer in pending state must never be invalidated or reset because device may read it at the moment.
/// Proving device is done with buffer requires nontrivial strategies.
/// Therefore moving buffer from pending state requires `unsafe` method.
#[derive(Clone, Copy, Debug, Default)]
pub struct PendingState<N>(N);

/// One-shot buffers move to invalid state after execution.
/// Invalidating any resource referenced in any command recorded to the buffer implicitly move it to the invalid state.
#[derive(Clone, Copy, Debug, Default)]
pub struct InvalidState;

/// States in which command buffer can de reset.
pub trait Resettable {}
impl Resettable for InitialState {}
impl<U> Resettable for RecordingState<U> {}
impl<U> Resettable for ExecutableState<U> {}
impl Resettable for InvalidState {}

/// Command buffer with this usage flag will move to invalid state after execution.
/// Resubmitting will require reset and rerecording commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct OneShot;

/// Command buffer with this usage flag will move back to executable state after execution.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiShot<S = ()>(pub S);

/// Additional flag for `MultiShot` that allows to resubmit buffer in pending state.
/// Note that resubmitting pending buffers can hurt performance.
#[derive(Clone, Copy, Debug, Default)]
pub struct SimultaneousUse;

/// Buffers with this usage flag must be secondary buffers executed entirely in render-pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPassContinue;

/// Trait implemented by all usage types.
pub trait Usage {
    /// State in which command buffer moves after completion.
    fn flags(&self) -> vk::CommandBufferUsageFlags;
}

impl Usage for OneShot {
    fn flags(&self) -> vk::CommandBufferUsageFlags {
        vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT
    }
}

impl Usage for MultiShot {
    fn flags(&self) -> vk::CommandBufferUsageFlags {
        vk::CommandBufferUsageFlags::empty()
    }
}

impl Usage for MultiShot<SimultaneousUse> {
    fn flags(&self) -> vk::CommandBufferUsageFlags {
        vk::CommandBufferUsageFlags::SIMULTANEOUS_USE
    }
}

/// Command buffer wrapper.
/// This wrapper defines state with usage, level and ability to be individually reset at type level.
/// This way many methods become safe.
#[derive(Debug)]
pub struct CommandBuffer<C, S, L = PrimaryLevel, R = NoIndividualReset> {
    raw: vk::CommandBuffer,
    capability: C,
    state: S,
    level: L,
    reset: R,
    family: FamilyIndex,
    relevant: Relevant,
}

impl<C, S, L, R> CommandBuffer<C, S, L, R> {
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
    pub unsafe fn from_raw(
        raw: vk::CommandBuffer,
        capability: C,
        state: S,
        level: L,
        reset: R,
        family: FamilyIndex,
    ) -> Self {
        CommandBuffer {
            raw,
            capability,
            state,
            level,
            reset,
            family,
            relevant: Relevant,
        }
    }

    /// Get raw command buffer handle.
    ///
    /// # Safety
    ///
    /// * Valid usage for command buffer must not be violated.
    /// Particularly command buffer must not change its state.
    /// Or `change_state` must be used to reflect accumulated change.
    pub unsafe fn raw(&self) -> vk::CommandBuffer {
        self.raw
    }

    /// Get raw command buffer handle.
    ///
    /// # Safety
    ///
    /// * Valid usage for command buffer must not be violated.
    pub unsafe fn into_raw(self) -> vk::CommandBuffer {
        self.relevant.dispose();
        self.raw
    }

    /// Change state of the command buffer.
    ///
    /// # Safety
    ///
    /// * This method must be used only to reflect state changed due to raw handle usage.
    pub unsafe fn change_state<U>(self, f: impl FnOnce(S) -> U) -> CommandBuffer<C, U, L, R> {
        CommandBuffer {
            raw: self.raw,
            capability: self.capability,
            state: f(self.state),
            level: self.level,
            reset: self.reset,
            family: self.family,
            relevant: self.relevant,
        }
    }

    /// Get buffers capability.
    pub fn capability(&self) -> C
    where
        C: Capability,
    {
        self.capability
    }
}

impl<C, R> CommandBuffer<C, InitialState, PrimaryLevel, R> {
    /// Begin recording command buffer.
    ///
    /// # Parameters
    ///
    /// `usage` - specifies usage of the command buffer. Possible types are `OneShot`, `MultiShot`.
    pub fn begin<U>(
        self,
        device: &impl DeviceV1_0,
        usage: U,
    ) -> CommandBuffer<C, RecordingState<U>, PrimaryLevel, R>
    where
        U: Usage,
    {
        unsafe {
            device
                .begin_command_buffer(
                    self.raw,
                    &vk::CommandBufferBeginInfo::builder()
                        .flags(usage.flags())
                        .build(),
                ).expect("Panic on OOM");

            self.change_state(|_| RecordingState(usage))
        }
    }
}

impl<C, U, R> CommandBuffer<C, RecordingState<U>, PrimaryLevel, R> {
    /// Finish recording command buffer.
    ///
    /// # Parameters
    pub fn finish(
        self,
        device: &impl DeviceV1_0,
    ) -> CommandBuffer<C, ExecutableState<U>, PrimaryLevel, R>
    where
        U: Usage,
    {
        unsafe {
            device.end_command_buffer(self.raw).expect("Panic on OOM");
            self.change_state(|RecordingState(usage)| ExecutableState(usage))
        }
    }
}

/// Structure contains command buffer ready for submission.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Submit {
    raw: vk::CommandBuffer,
    family: FamilyIndex,
}

impl Submit {
    /// Get family this submit is associated with.
    pub fn family(&self) -> FamilyIndex {
        self.family
    }

    /// Get raw command buffer.
    pub fn raw(&self) -> vk::CommandBuffer {
        self.raw
    }
}

impl<C, S, R> CommandBuffer<C, ExecutableState<S>, PrimaryLevel, R> {
    /// produce `Submit` object that can be used to populate submission.
    pub fn submit_once(
        self,
    ) -> (
        Submit,
        CommandBuffer<C, PendingState<InvalidState>, PrimaryLevel, R>,
    ) {
        let buffer = unsafe { self.change_state(|_| PendingState(InvalidState)) };

        let submit = Submit {
            raw: buffer.raw,
            family: buffer.family,
        };

        (submit, buffer)
    }
}

impl<C, S, R> CommandBuffer<C, ExecutableState<MultiShot<S>>, PrimaryLevel, R> {
    /// Produce `Submit` object that can be used to populate submission.
    pub fn submit(
        self,
    ) -> (
        Submit,
        CommandBuffer<C, PendingState<ExecutableState<MultiShot<S>>>, PrimaryLevel, R>,
    ) {
        let buffer = unsafe { self.change_state(|state| PendingState(state)) };

        let submit = Submit {
            raw: buffer.raw,
            family: buffer.family,
        };

        (submit, buffer)
    }
}

impl<C, N, L, R> CommandBuffer<C, PendingState<N>, L, R> {
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
    /// [wait]: ../ash/version/trait.DeviceV1_0.html#method.wait_for_fences
    /// [`Fence`]: ../ash/vk/struct.Fence.html
    /// [submitting]: ../ash/version/trait.DeviceV1_0.html#method.queue_submit
    pub unsafe fn complete(self) -> CommandBuffer<C, N, L, R> {
        self.change_state(|PendingState(state)| state)
    }

    /// Release command buffer.
    ///
    /// # Safety
    ///
    /// * It must be owned by `OwningCommandPool`
    /// TODO: Use lifetimes to tie `CommandCommand buffer` to `OwningCommandPool`.
    pub unsafe fn release(self) {
        self.relevant.dispose();
    }
}

impl<C, S, L> CommandBuffer<C, S, L, IndividualReset>
where
    S: Resettable,
{
    /// Reset command buffer.
    pub fn reset(self) -> CommandBuffer<C, InitialState, L, IndividualReset> {
        unsafe { self.change_state(|_| InitialState) }
    }
}

impl<C, S, L> CommandBuffer<C, S, L>
where
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
    pub unsafe fn mark_reset(self) -> CommandBuffer<C, InitialState, L> {
        self.change_state(|_| InitialState)
    }
}
