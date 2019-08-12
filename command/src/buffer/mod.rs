//! Command buffer module docs.

mod encoder;
mod level;
mod reset;
mod state;
mod submit;
mod usage;

use {
    crate::{
        capability::{Capability, Supports},
        family::FamilyId,
    },
    rendy_core::hal::Backend,
};

pub use self::{encoder::*, level::*, reset::*, state::*, submit::*, usage::*};

/// Command buffer wrapper.
/// This wrapper defines state with usage, level and ability to be individually reset at type level.
/// This way many methods become safe.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct CommandBuffer<B: Backend, C, S, L = PrimaryLevel, R = NoIndividualReset> {
    #[derivative(Debug = "ignore")]
    raw: std::ptr::NonNull<B::CommandBuffer>,
    capability: C,
    state: S,
    level: L,
    reset: R,
    family: FamilyId,
    relevant: relevant::Relevant,
}

family_owned!(CommandBuffer<B, C, S, L, R>);

unsafe impl<B, C, S, L, R> Send for CommandBuffer<B, C, S, L, R>
where
    B: Backend,
    B::CommandBuffer: Send,
    C: Send,
    S: Send,
    L: Send,
    R: Send,
    FamilyId: Send,
    relevant::Relevant: Send,
{
}

unsafe impl<B, C, S, L, R> Sync for CommandBuffer<B, C, S, L, R>
where
    B: Backend,
    B::CommandBuffer: Sync,
    C: Sync,
    S: Sync,
    L: Sync,
    R: Sync,
    FamilyId: Sync,
    relevant::Relevant: Sync,
{
}

impl<B, C, S, L, R> CommandBuffer<B, C, S, L, R>
where
    B: Backend,
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
        raw: B::CommandBuffer,
        capability: C,
        state: S,
        level: L,
        reset: R,
        family: FamilyId,
    ) -> Self {
        CommandBuffer {
            raw: std::ptr::NonNull::new_unchecked(Box::into_raw(Box::new(raw))),
            capability,
            state,
            level,
            reset,
            family,
            relevant: relevant::Relevant,
        }
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

    /// Get buffers family.
    pub fn family(&self) -> FamilyId {
        self.family
    }

    /// Convert capability level.
    pub fn with_queue_type(self) -> CommandBuffer<B, rendy_core::hal::queue::QueueType, S, L, R>
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
            relevant: self.relevant,
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
                relevant: self.relevant,
            })
        } else {
            Err(self)
        }
    }
}

/// Begin info for specific level and render pass relation.
pub unsafe trait BeginInfo<'a, B: Backend, L> {
    /// Pass relation type.
    type PassRelation: RenderPassRelation<L>;

    /// Get command buffer inheritance info.
    fn inheritance_info(self) -> rendy_core::hal::command::CommandBufferInheritanceInfo<'a, B>;
}

unsafe impl<'a, B, L> BeginInfo<'a, B, L> for ()
where
    B: Backend,
    L: Level,
{
    type PassRelation = OutsideRenderPass;

    fn inheritance_info(self) -> rendy_core::hal::command::CommandBufferInheritanceInfo<'a, B> {
        rendy_core::hal::command::CommandBufferInheritanceInfo::default()
    }
}

unsafe impl<'a, B> BeginInfo<'a, B, SecondaryLevel> for rendy_core::hal::pass::Subpass<'a, B>
where
    B: Backend,
{
    type PassRelation = RenderPassContinue;

    fn inheritance_info(self) -> rendy_core::hal::command::CommandBufferInheritanceInfo<'a, B> {
        rendy_core::hal::command::CommandBufferInheritanceInfo {
            subpass: Some(self),
            framebuffer: None,
            ..rendy_core::hal::command::CommandBufferInheritanceInfo::default()
        }
    }
}

unsafe impl<'a, B, F> BeginInfo<'a, B, SecondaryLevel>
    for (rendy_core::hal::pass::Subpass<'a, B>, Option<&'a F>)
where
    B: Backend,
    F: std::borrow::Borrow<B::Framebuffer>,
{
    type PassRelation = RenderPassContinue;

    fn inheritance_info(self) -> rendy_core::hal::command::CommandBufferInheritanceInfo<'a, B> {
        rendy_core::hal::command::CommandBufferInheritanceInfo {
            subpass: Some(self.0),
            framebuffer: self.1.map(F::borrow),
            ..rendy_core::hal::command::CommandBufferInheritanceInfo::default()
        }
    }
}

unsafe impl<'a, B, F> BeginInfo<'a, B, SecondaryLevel> for (rendy_core::hal::pass::Subpass<'a, B>, &'a F)
where
    B: Backend,
    F: std::borrow::Borrow<B::Framebuffer>,
{
    type PassRelation = RenderPassContinue;

    fn inheritance_info(self) -> rendy_core::hal::command::CommandBufferInheritanceInfo<'a, B> {
        rendy_core::hal::command::CommandBufferInheritanceInfo {
            subpass: Some(self.0),
            framebuffer: Some(self.1.borrow()),
            ..rendy_core::hal::command::CommandBufferInheritanceInfo::default()
        }
    }
}

impl<B, C, L, R> CommandBuffer<B, C, InitialState, L, R>
where
    B: Backend,
{
    /// Begin recording command buffer.
    ///
    /// # Parameters
    ///
    /// `usage` - specifies usage of the command buffer. Possible types are `OneShot`, `MultiShot`.
    pub fn begin<'a, U, P>(
        mut self,
        usage: U,
        info: impl BeginInfo<'a, B, L, PassRelation = P>,
    ) -> CommandBuffer<B, C, RecordingState<U, P>, L, R>
    where
        U: Usage,
        P: RenderPassRelation<L>,
    {
        let pass_relation = P::default();
        unsafe {
            rendy_core::hal::command::CommandBuffer::begin(
                self.raw(),
                usage.flags() | pass_relation.flags(),
                info.inheritance_info(),
            );

            self.change_state(|_| RecordingState(usage, pass_relation))
        }
    }
}

impl<'a, B, C, U, P, L, R> CommandBuffer<B, C, RecordingState<U, P>, L, R>
where
    B: Backend,
{
    /// Finish recording command buffer.
    pub fn finish(mut self) -> CommandBuffer<B, C, ExecutableState<U, P>, L, R> {
        unsafe {
            rendy_core::hal::command::CommandBuffer::finish(self.raw());

            self.change_state(|s| ExecutableState(s.0, s.1))
        }
    }
}

impl<B, C, N, L, R> CommandBuffer<B, C, PendingState<N>, L, R>
where
    B: Backend,
{
    /// Mark command buffer as complete.
    ///
    /// # Safety
    ///
    /// None of [`Submit`] instances created from this `CommandBuffer` are alive.
    ///
    /// If this is `PrimaryLevel` buffer then
    /// for each command queue where [`Submit`] instance (created from this `CommandBuffer`)
    /// was submitted at least one [`Fence`] submitted within same `Submission` or later in unset state was `set`.
    ///
    /// If this is `Secondary` buffer then
    /// all primary command buffers where [`Submit`] instance (created from this `CommandBuffer`)
    /// was submitted must be complete.
    ///
    /// [`Submit`]: struct.Submit
    /// [waiting]: ..gfx_hal/device/trait.Device.html#method.wait_for_fences
    /// [`Fence`]: ..gfx_hal/trait.Backend.html#associatedtype.Fence
    /// [submitted]: ..gfx_hal/queue/struct.CommandQueue.html#method.submit
    pub unsafe fn mark_complete(self) -> CommandBuffer<B, C, N, L, R> {
        self.change_state(|PendingState(state)| state)
    }
}

impl<B, C, S, L> CommandBuffer<B, C, S, L, IndividualReset>
where
    B: Backend,
    S: Resettable,
{
    /// Reset command buffer.
    pub fn reset(self) -> CommandBuffer<B, C, InitialState, L, IndividualReset> {
        unsafe { self.change_state(|_| InitialState) }
    }
}

impl<B, C, S, L> CommandBuffer<B, C, S, L>
where
    B: Backend,
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

impl<B, C, S, L, R> CommandBuffer<B, C, S, L, R>
where
    B: Backend,
    S: Resettable,
{
    /// Dispose of command buffer wrapper releasing raw comman buffer value.
    /// This function is intended to be used to deallocate command buffer.
    pub fn into_raw(self) -> B::CommandBuffer {
        self.relevant.dispose();
        unsafe {
            // state guarantees that raw command buffer is not shared.
            *Box::from_raw(self.raw.as_ptr())
        }
    }

    /// Get raw command buffer handle.
    pub fn raw(&mut self) -> &mut B::CommandBuffer {
        unsafe {
            // state guarantees that raw command buffer is not shared.
            self.raw.as_mut()
        }
    }
}
