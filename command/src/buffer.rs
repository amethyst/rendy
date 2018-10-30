//! Buffer module docs.

use std::borrow::Borrow;
use ash::{
    version::DeviceV1_0,
    vk::{
        CommandBuffer,
        CommandBufferLevel,
        CommandBufferUsageFlags,
        CommandPoolCreateFlags,
        CommandBufferBeginInfo,
    },
};
use relevant::Relevant;

use crate::family::FamilyIndex;

/// Command buffers of this level can be submitted to the command queues.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrimaryLevel;

/// Command buffers of this level can be executed as part of the primary buffers.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecondaryLevel;

/// Command buffer level.
pub trait Level: Copy {
    /// Get raw level value.
    fn level(&self) -> CommandBufferLevel;
}

impl Level for PrimaryLevel {
    fn level(&self) -> CommandBufferLevel {
        CommandBufferLevel::PRIMARY
    }
}

impl Level for SecondaryLevel {
    fn level(&self) -> CommandBufferLevel {
        CommandBufferLevel::SECONDARY
    }
}

impl Level for CommandBufferLevel {
    fn level(&self) -> CommandBufferLevel {
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
    fn flags(&self) -> CommandPoolCreateFlags;
}

impl Reset for IndividualReset {
    fn flags(&self) -> CommandPoolCreateFlags {
        CommandPoolCreateFlags::RESET_COMMAND_BUFFER
    }
}

impl Reset for NoIndividualReset {
    fn flags(&self) -> CommandPoolCreateFlags {
        CommandPoolCreateFlags::empty()
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
/// Buffer in pending state must never be invalidated or reset because device may read it at the moment.
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

/// Buffer with this usage flag will move to invalid state after execution.
/// Resubmitting will require reset and rerecording commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct OneShot;

/// Buffer with this usage flag will move back to executable state after execution.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiShot<S = ()>(S);

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
    fn flags(&self) -> CommandBufferUsageFlags;
}

impl Usage for OneShot {
    fn flags(&self) -> CommandBufferUsageFlags {
        CommandBufferUsageFlags::ONE_TIME_SUBMIT
    }
}

impl Usage for MultiShot {
    fn flags(&self) -> CommandBufferUsageFlags {
        CommandBufferUsageFlags::empty()
    }
}

impl Usage for MultiShot<SimultaneousUse> {
    fn flags(&self) -> CommandBufferUsageFlags {
        CommandBufferUsageFlags::SIMULTANEOUS_USE
    }
}

/// Command buffer wrapper.
/// This wrapper defines state with usage, level and ability to be individually reset at type level.
/// This way many methods become safe.
#[derive(Debug)]
pub struct Buffer<C, S, L, R = NoIndividualReset> {
    raw: CommandBuffer,
    capability: C,
    state: S,
    level: L,
    reset: R,
    family: FamilyIndex,
    relevant: Relevant,
}

impl<C, S, L, R> Buffer<C, S, L, R> {
    /// Wrap raw buffer handle.
    pub unsafe fn from_raw(
        raw: CommandBuffer,
        capability: C,
        state: S,
        level: L,
        reset: R,
        family: FamilyIndex,
    ) -> Self {
        Buffer {
            raw,
            capability,
            state,
            level,
            reset,
            family,
            relevant: Relevant,
        }
    }

    /// Get raw CommandBuffer
    pub unsafe fn raw(&self) -> CommandBuffer {
        self.raw
    }

    ///
    pub unsafe fn change_state<U>(self, f: impl FnOnce(S) -> U) -> Buffer<C, U, L, R> {
        Buffer {
            raw: self.raw,
            capability: self.capability,
            state: f(self.state),
            level: self.level,
            reset: self.reset,
            family: self.family,
            relevant: self.relevant,
        }
    }
}

impl<C, R> Buffer<C, InitialState, PrimaryLevel, R> {
    /// Begin recording command buffer.
    ///
    /// # Parameters
    ///
    /// `usage` - specifies usage of the command buffer. Possible types are `OneShot`, `MultiShot`.
    pub fn begin<U>(self, usage: U, device: &impl DeviceV1_0) -> Buffer<C, RecordingState<U>, PrimaryLevel, R>
    where
        U: Usage,
    {
        unsafe {
            device.begin_command_buffer(
                self.raw,
                &CommandBufferBeginInfo::builder()
                    .flags(usage.flags())
                    .build()
            ).expect("Panic on OOM");

            self.change_state(|_| RecordingState(usage))
        }        
    }
}

impl<C, U, R> Buffer<C, RecordingState<U>, PrimaryLevel, R> {
    /// Finish recording command buffer.
    ///
    /// # Parameters
    pub fn finish(self, device: &impl DeviceV1_0) -> Buffer<C, ExecutableState<U>, PrimaryLevel, R>
    where
        U: Usage,
    {
        unsafe {
            device.end_command_buffer(self.raw)
                .expect("Panic on OOM");
            self.change_state(|RecordingState(usage)| ExecutableState(usage))
        }
    }
}

/// Structure contains command buffer ready for submission.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Submit {
    raw: CommandBuffer,
    family: FamilyIndex,
}

impl Submit {
    /// Get family this submit is associated with.
    pub fn family(&self) -> FamilyIndex {
        self.family
    }

    /// Get raw command buffer.
    pub fn raw(&self) -> CommandBuffer {
        self.raw
    }
}

impl<C, R> Buffer<C, ExecutableState<OneShot>, PrimaryLevel, R> {
    /// produce `Submit` object that can be used to populate submission.
    pub fn submit_once(self) -> (
        Submit,
        Buffer<C, PendingState<InvalidState>, PrimaryLevel, R>,
    ) {
        let buffer = unsafe {

            self.change_state(|_| PendingState(InvalidState))
        };

        let submit = Submit {
            raw: buffer.raw,
            family: buffer.family,
        };

        (submit, buffer)
    }
}

impl<C, S, R> Buffer<C, ExecutableState<MultiShot<S>>, PrimaryLevel, R> {
    /// Produce `Submit` object that can be used to populate submission.
    pub fn submit(self) -> (
        Submit,
        Buffer<C, PendingState<ExecutableState<MultiShot<S>>>, PrimaryLevel, R>,
    ) {
        let buffer = unsafe {
            self.change_state(|state| PendingState(state))
        };

        let submit = Submit {
            raw: buffer.raw,
            family: buffer.family,
        };

        (submit, buffer)
    }
}

impl<C, N, L, R> Buffer<C, PendingState<N>, L, R> {
    /// Mark command buffer as complete.
    ///
    /// # Safety
    ///
    /// User must ensure that recorded commands are complete.
    pub unsafe fn complete(self) -> Buffer<C, N, L, R> {
        self.change_state(|PendingState(state)| state)
    }

    /// Release command buffer.
    ///
    /// # Safety
    ///
    /// It must be owned by `OwningPool`
    pub unsafe fn release(self) {
        self.relevant.dispose();
    }
}

impl<C, S, L> Buffer<C, S, L, IndividualReset>
where
    S: Resettable,
{
    /// Reset command buffer.
    pub fn reset(self) -> Buffer<C, InitialState, L, IndividualReset> {
        unsafe {
            self.change_state(|_| InitialState)
        }
    }
}

impl<C, S, L> Buffer<C, S, L>
where
    S: Resettable,
{
    /// Reset command buffer.
    ///
    /// # Safety
    ///
    /// Mark command buffer as reset.
    /// User must reset buffer via command pool and call this method for all commands buffers affected.
    pub unsafe fn mark_reset(self) -> Buffer<C, InitialState, L> {
        self.change_state(|_| InitialState)
    }
}
