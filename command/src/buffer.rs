//! Buffer module docs.

use ash::vk::{CommandBuffer, CommandBufferLevel, CommandBufferUsageFlags};
use relevant::Relevant;
use std::borrow::Borrow;

use crate::family::FamilyId;

/// Command buffers of this level can be submitted to the command queues.
#[derive(Clone, Copy, Debug)]
pub struct PrimaryLevel;

/// Command buffers of this level can be executed as part of the primary buffers.
#[derive(Clone, Copy, Debug)]
pub struct SecondaryLevel;

/// Command buffer level.
pub trait Level {
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
/// Without this flag buffer can be reset only together with all other buffers from pool.
#[derive(Clone, Copy, Debug)]
pub struct IndividualReset;

/// Command buffer state in which all buffers start.
/// Resetting also moves buffer to this state.
#[derive(Clone, Copy, Debug)]
pub struct InitialState;

/// Command buffer in recording state could be populated with commands.
#[derive(Clone, Copy, Debug)]
pub struct RecordingState<U>(U);

/// Command buffer in executable state can be submitted.
#[derive(Clone, Copy, Debug)]
pub struct ExecutableState<U>(U);

/// Command buffer in pending state are submitted to the device.
/// Buffer in pending state must never be invalidated or reset because device may read it at the moment.
/// Proving device is done with buffer requires nontrivial strategies.
/// Therefore moving buffer from pending state requires `unsafe` method.
#[derive(Clone, Copy, Debug)]
pub struct PendingState<N>(N);

/// One-shot buffers move to invalid state after execution.
/// Invalidating any resource referenced in any command recorded to the buffer implicitly move it to the invalid state.
#[derive(Clone, Copy, Debug)]
pub struct InvalidState;

/// States in which command buffer can be destroyed.
pub trait Droppable {}
impl Droppable for InitialState {}
impl<U> Droppable for RecordingState<U> {}
impl<U> Droppable for ExecutableState<U> {}
impl Droppable for InvalidState {}

/// States in which command buffer can de reset.
pub trait Resettable: Droppable {}
impl<U> Resettable for RecordingState<U> {}
impl<U> Resettable for ExecutableState<U> {}
impl Resettable for InvalidState {}

/// Buffer with this usage flag will move to invalid state after execution.
/// Resubmitting will require reset and rerecording commands.
#[derive(Clone, Copy, Debug)]
pub struct OneShot;

/// Buffer with this usage flag will move back to executable state after execution.
#[derive(Clone, Copy, Debug)]
pub struct MultiShot<S = ()>(S);

/// Additional flag for `MultiShot` that allows to resubmit buffer in pending state.
/// Note that resubmitting pending buffers can hurt performance.
#[derive(Clone, Copy, Debug)]
pub struct SimultaneousUse;

/// Buffers with this usage flag must be secondary buffers executed entirely in render-pass.
#[derive(Clone, Copy, Debug)]
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
pub struct Buffer<C, S, L, R = ()> {
    inner: CommandBuffer,
    capability: C,
    state: S,
    level: L,
    reset: R,
    family: FamilyId,
    relevant: Relevant,
}

impl<C, S, L, R> Buffer<C, S, L, R> {
    /// Get raw CommandBuffer
    pub unsafe fn raw(&self) -> CommandBuffer {
        self.inner
    }
}

impl<C, R> Buffer<C, InitialState, PrimaryLevel, R> {
    /// Begin recording command buffer.
    ///
    /// # Parameters
    ///
    /// `usage` - specifies usage of the command buffer. Possible types are `OneShot`, `MultiShot`.
    pub fn begin<U>(self, usage: U) -> Buffer<C, RecordingState<U>, PrimaryLevel, R>
    where
        U: Usage,
    {
        unimplemented!()
    }
}

/// Structure contains command buffer ready for submission.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Submit {
    raw: CommandBuffer,
    family: FamilyId,
}

impl Submit {
    /// Get family this submit is associated with.
    pub fn family(&self) -> FamilyId {
        self.family
    }

    /// Get raw command buffer.
    pub fn raw(&self) -> CommandBuffer {
        self.raw
    }
}

impl<C, R> Buffer<C, ExecutableState<OneShot>, PrimaryLevel, R> {
    /// produce `Submit` object that can be used to populate submission.
    pub fn submit_once(
        self,
    ) -> (
        Submit,
        Buffer<C, PendingState<InvalidState>, PrimaryLevel, R>,
    ) {
        unimplemented!()
    }
}

impl<C, S, R> Buffer<C, ExecutableState<MultiShot<S>>, PrimaryLevel, R> {
    /// Produce `Submit` object that can be used to populate submission.
    pub fn submit(
        self,
    ) -> (
        Submit,
        Buffer<C, PendingState<ExecutableState<MultiShot<S>>>, PrimaryLevel, R>,
    ) {
        unimplemented!()
    }
}

impl<C, N, L, R> Buffer<C, PendingState<N>, L, R> {
    /// Mark command buffer as complete.
    ///
    /// # Safety
    ///
    /// User must ensure that recorded commands are complete.
    pub unsafe fn complete(self) -> Buffer<C, N, L, R> {
        unimplemented!()
    }
}

impl<C, S, L> Buffer<C, S, L, IndividualReset>
where
    S: Resettable,
{
    /// Reset command buffer.
    pub fn reset(self) -> Buffer<C, InitialState, L, IndividualReset> {
        unimplemented!()
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
        unimplemented!()
    }
}
