//! Command buffer module docs.

use crate::{
    capability::{Capability, Supports},
};

/// Command buffers of this level can be submitted to the command queues.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrimaryLevel;

/// Command buffers of this level can be executed as part of the primary buffers.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecondaryLevel;

/// Command buffer level.
pub trait Level: Copy {
    /// Get raw level value.
    fn level(&self) -> gfx_hal::command::RawLevel;
}

impl Level for PrimaryLevel {
    fn level(&self) -> gfx_hal::command::RawLevel {
        gfx_hal::command::RawLevel::Primary
    }
}

impl Level for SecondaryLevel {
    fn level(&self) -> gfx_hal::command::RawLevel {
        gfx_hal::command::RawLevel::Secondary
    }
}

impl Level for gfx_hal::command::RawLevel {
    fn level(&self) -> gfx_hal::command::RawLevel {
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
pub trait Reset: Copy + Default {
    /// Get flags for reset parameter.
    fn flags(&self) -> gfx_hal::pool::CommandPoolCreateFlags;
}

impl Reset for IndividualReset {
    fn flags(&self) -> gfx_hal::pool::CommandPoolCreateFlags {
        gfx_hal::pool::CommandPoolCreateFlags::RESET_INDIVIDUAL
    }
}

impl Reset for NoIndividualReset {
    fn flags(&self) -> gfx_hal::pool::CommandPoolCreateFlags {
        gfx_hal::pool::CommandPoolCreateFlags::empty()
    }
}

/// Command buffer state in which all buffers start.
/// Resetting also moves buffer to this state.
#[derive(Clone, Copy, Debug, Default)]
pub struct InitialState;

/// Command buffer in recording state could be populated with commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct RecordingState<U = MultiShot>(U);

/// Command buffer in executable state can be submitted.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExecutableState<U = MultiShot>(U);

/// Command buffer in pending state are submitted to the device.
/// Command buffer in pending state must never be invalidated or reset because device may read it at the moment.
/// Proving device is done with buffer requires nontrivial strategies.
/// Therefore moving buffer from pending state requires `unsafe` method.
#[derive(Clone, Copy, Debug, Default)]
pub struct PendingState<N = ExecutableState>(N);

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
pub struct OneShot<C = ()>(pub C);

/// Command buffer with this usage flag will move back to executable state after execution.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiShot<S = (), P = ()>(pub S, pub P);

/// Additional flag for `MultiShot` that allows to resubmit buffer in pending state.
/// Note that resubmitting pending buffers can hurt performance.
#[derive(Clone, Copy, Debug, Default)]
pub struct SimultaneousUse;

/// Buffers with this usage flag must be secondary buffers executed entirely in render-pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPassContinue;

/// Trait implemented by all usage types.
pub trait Usage: Copy + Default {
    /// State in which command buffer moves after completion.
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags;
}

impl Usage for OneShot {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::ONE_TIME_SUBMIT
    }
}

impl Usage for OneShot<RenderPassContinue> {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::ONE_TIME_SUBMIT | gfx_hal::command::CommandBufferFlags::RENDER_PASS_CONTINUE
    }
}

impl Usage for MultiShot {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::empty()
    }
}

impl Usage for MultiShot<SimultaneousUse> {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::SIMULTANEOUS_USE
    }
}

impl Usage for MultiShot<(), RenderPassContinue> {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::RENDER_PASS_CONTINUE
    }
}

impl Usage for MultiShot<SimultaneousUse, RenderPassContinue> {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::SIMULTANEOUS_USE | gfx_hal::command::CommandBufferFlags::RENDER_PASS_CONTINUE
    }
}

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

    /// Convert capability level.
    pub fn with_value_capability(self) -> CommandBuffer<B, gfx_hal::QueueType, S, L, R>
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

impl<B, C, S, L, R> CommandBuffer<B, C, S, L, R>
where
    B: gfx_hal::Backend,
{
    /// Get raw command buffer handle.
    ///
    /// # Safety
    ///
    /// * Valid usage for command buffer must not be violated.
    pub unsafe fn into_raw(self) -> B::CommandBuffer {
        self.raw
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
    pub fn begin<U>(
        mut self,
        usage: U,
    ) -> CommandBuffer<B, C, RecordingState<U>, L, R>
    where
        U: Usage,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::begin(
                &mut self.raw,
                usage.flags(),
                gfx_hal::command::CommandBufferInheritanceInfo::default(),
            );

            self.change_state(|_| RecordingState(usage))
        }
    }
}

impl<B, C, U, L, R> CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: gfx_hal::Backend,
{
    /// Finish recording command buffer.
    ///
    /// # Parameters
    pub fn finish(
        mut self,
    ) -> CommandBuffer<B, C, ExecutableState<U>, L, R>
    where
        U: Usage,
    {
        unsafe {
            gfx_hal::command::RawCommandBuffer::finish(&mut self.raw);
            self.change_state(|RecordingState(usage)| ExecutableState(usage))
        }
    }
}

/// Structure contains command buffer ready for submission.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Submit<'a, B: gfx_hal::Backend, S = (), P = (), L = PrimaryLevel> {
    raw: B::CommandBuffer,
    family: gfx_hal::queue::QueueFamilyId,
    pass_continue: P,
    simultaneous: S,
    level: L,
    marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, B, S, P, L> Submit<'a, B, S, P, L>
where
    B: gfx_hal::Backend,
{
    /// Get family this submit is associated with.
    pub fn family(&self) -> gfx_hal::queue::QueueFamilyId {
        self.family
    }

    /// Get raw command buffer.
    pub fn raw(&self) -> &B::CommandBuffer {
        &self.raw
    }

    /// Get raw command buffer.
    pub fn into_raw(self) -> B::CommandBuffer {
        self.raw
    }
}

/// Submittable object.
pub unsafe trait Submittable<B: gfx_hal::Backend> {

    /// Get family that this submittable is belong to.
    fn family(&self) -> gfx_hal::queue::QueueFamilyId;

    /// Get raw command buffer.
    fn raw(&self) -> &B::CommandBuffer;
}

unsafe impl<'a, B> Submittable<B> for Submit<'a, B>
where
    B: gfx_hal::Backend,
{
    fn family(&self) -> gfx_hal::queue::QueueFamilyId {
        self.family
    }

    fn raw(&self) -> &B::CommandBuffer {
        &self.raw
    }
}

unsafe impl<'a, 'b, B> Submittable<B> for &'a Submit<'b, B, SimultaneousUse>
where
    B: gfx_hal::Backend,
{
    fn family(&self) -> gfx_hal::queue::QueueFamilyId {
        self.family
    }

    fn raw(&self) -> &B::CommandBuffer {
        &self.raw
    }
}

impl<B, C, P, L, R> CommandBuffer<B, C, ExecutableState<OneShot<P>>, L, R>
where
    B: gfx_hal::Backend,
    P: Copy,
    L: Copy,
{
    /// Produce `Submit` object that can be used to populate submission.
    pub fn submit(
        self,
    ) -> (
        Submit<'static, B, (), P, L>,
        CommandBuffer<B, C, PendingState<InvalidState>, L, R>,
    ) {
        let OneShot(pass_continue) = self.state.0;
        let level = self.level;

        let buffer = unsafe { self.change_state(|_| PendingState(InvalidState)) };

        let submit = Submit {
            raw: buffer.raw.clone(),
            family: buffer.family,
            pass_continue,
            simultaneous: (),
            level,
            marker: std::marker::PhantomData,
        };

        (submit, buffer)
    }
}

impl<B, C, S, P, L, R> CommandBuffer<B, C, ExecutableState<MultiShot<S, P>>, L, R>
where
    B: gfx_hal::Backend,
    P: Copy,
    S: Copy,
    L: Copy,
{
    /// Produce `Submit` object that can be used to populate submission.
    pub fn submit(
        self,
    ) -> (
        Submit<'static, B, S, P, L>,
        CommandBuffer<B, C, PendingState<ExecutableState<MultiShot<S, P>>>, L, R>,
    ) {
        let MultiShot(simultaneous, pass_continue) = self.state.0;
        let level = self.level;

        let buffer = unsafe { self.change_state(|state| PendingState(state)) };

        let submit = Submit {
            raw: buffer.raw.clone(),
            family: buffer.family,
            pass_continue,
            simultaneous,
            level,
            marker: std::marker::PhantomData,
        };

        (submit, buffer)
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

impl<B, C, U, L, R> CommandBuffer<B, C, RecordingState<U>, L, R>
where
    B: gfx_hal::Backend,
    C: Capability,
{
    /// Acquire encoder for command buffer.
    pub fn encoder<'a>(&'a mut self) -> CommandBufferEncoder<'a, B, C, C, U, L, R> {
        self.into()
    }
}

/// Command encoder.
///
///
#[derive(Debug)]
pub struct CommandBufferEncoder<'a, B: gfx_hal::Backend, C: 'a, X, U: 'a, L: 'a, R: 'a> {
    buffer: &'a mut CommandBuffer<B, C, RecordingState<U>, L, R>,
    capability: X,
}

impl<'a, B, C, X, U, L, R> CommandBufferEncoder<'a, B, C, X, U, L, R>
where
    B: gfx_hal::Backend,
{
    /// Convert capability level
    pub fn with_capability<Y>(self) -> Result<CommandBufferEncoder<'a, B, C, Y, U, L, R>, Self>
    where
        X: Supports<Y>,
    {
        if let Some(capability) = self.capability.supports() {
            Ok(CommandBufferEncoder {
                buffer: self.buffer,
                capability,
            })
        } else {
            Err(self)
        }
    }
}

impl<'a, B, C, U, L, R> From<&'a mut CommandBuffer<B, C, RecordingState<U>, L, R>> for CommandBufferEncoder<'a, B, C, C, U, L, R>
where
    B: gfx_hal::Backend,
    C: Capability,
{
    fn from(buffer: &'a mut CommandBuffer<B, C, RecordingState<U>, L, R>) -> Self {
        CommandBufferEncoder {
            capability: buffer.capability(),
            buffer,
        }
    }
}
