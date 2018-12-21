
use super::{
    CommandBuffer,
    level::PrimaryLevel,
    state::{ExecutableState, PendingState, InvalidState},
    usage::{OneShot, MultiShot, SimultaneousUse, NoSimultaneousUse, OutsideRenderPass},
};

/// Structure contains command buffer ready for submission.
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Submit<B: gfx_hal::Backend, S = NoSimultaneousUse, L = PrimaryLevel, P = OutsideRenderPass> {
    #[derivative(Debug = "ignore")]
    raw: *const B::CommandBuffer,
    family: gfx_hal::queue::QueueFamilyId,
    pass_continue: P,
    simultaneous: S,
    level: L,
}

unsafe impl<B, S, L, P> Send for Submit<B, S, L, P>
where
    B: gfx_hal::Backend,
    B::CommandBuffer: Sync,
{}
unsafe impl<B, S, L, P> Sync for Submit<B, S, L, P>
where
    B: gfx_hal::Backend,
    B::CommandBuffer: Sync,
{}

/// Submittable object.
/// Values that implement this trait can be submitted to the queues
/// or executed as part of primary buffers (in case of `Submittable<B, SecondaryLevel>`).
pub unsafe trait Submittable<B: gfx_hal::Backend, L = PrimaryLevel, P = OutsideRenderPass> {
    /// Get family that this submittable is belong to.
    fn family(&self) -> gfx_hal::queue::QueueFamilyId;

    /// Get raw command buffer.
    /// 
    /// # Safety
    /// 
    /// The command buffer is returned as raw pointer
    /// because its lifetime is not tied to `Submittable` instance lifetime
    /// but rather to original `CommandBuffer`.
    /// The command buffer cannot be freed before commands are complete
    /// which cannot be done before they are submitted.
    /// Dereferencing this pointer to perform submission is totally safe.
    /// On the other hand calling `CommandBuffer::mark_complete` (which must be done so buffer may be freed)
    /// before this pointer used for submission is considered an error.
    fn raw(self) -> *const B::CommandBuffer;
}

unsafe impl<B, S, L, P> Submittable<B, L, P> for Submit<B, S, L, P>
where
    B: gfx_hal::Backend,
{
    fn family(&self) -> gfx_hal::queue::QueueFamilyId {
        self.family
    }

    fn raw(self) -> *const B::CommandBuffer {
        self.raw
    }
}

unsafe impl<'a, B, L, P> Submittable<B, L, P> for &'a Submit<B, SimultaneousUse, L, P>
where
    B: gfx_hal::Backend,
{
    fn family(&self) -> gfx_hal::queue::QueueFamilyId {
        self.family
    }

    fn raw(self) -> *const B::CommandBuffer {
        self.raw
    }
}

impl<B, C, P, L, R> CommandBuffer<B, C, ExecutableState<OneShot, P>, L, R>
where
    B: gfx_hal::Backend,
    P: Copy,
    L: Copy,
{
    /// Produce `Submit` object that can be used to populate submission.
    pub fn submit_once(
        self,
    ) -> (
        Submit<B, NoSimultaneousUse, L, P>,
        CommandBuffer<B, C, PendingState<InvalidState>, L, R>,
    ) {
        let pass_continue = self.state.1;
        let level = self.level;

        let buffer = unsafe { self.change_state(|_| PendingState(InvalidState)) };

        let submit = Submit {
            raw: &*buffer.raw,
            family: buffer.family,
            pass_continue,
            simultaneous: NoSimultaneousUse,
            level,
        };

        (submit, buffer)
    }
}

impl<B, C, S, L, P, R> CommandBuffer<B, C, ExecutableState<MultiShot<S>, P>, L, R>
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
        Submit<B, S, L, P>,
        CommandBuffer<B, C, PendingState<ExecutableState<MultiShot<S>, P>>, L, R>,
    ) {
        let MultiShot(simultaneous) = self.state.0;
        let pass_continue = self.state.1;
        let level = self.level;

        let buffer = unsafe { self.change_state(|state| PendingState(state)) };

        let submit = Submit {
            raw: &*buffer.raw,
            family: buffer.family,
            pass_continue,
            simultaneous,
            level,
        };

        (submit, buffer)
    }
}
