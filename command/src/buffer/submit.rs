
use super::{
    CommandBuffer,
    level::PrimaryLevel,
    state::{ExecutableState, PendingState, InvalidState},
    usage::{OneShot, MultiShot, SimultaneousUse},
};

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

unsafe impl<'a, B, S, P, L> Submittable<B> for Submit<'a, B, S, P, L>
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

unsafe impl<'a, 'b, B, P, L> Submittable<B> for &'a Submit<'b, B, SimultaneousUse, P, L>
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

impl<B, C, P, L, R> CommandBuffer<B, C, ExecutableState<OneShot, P>, L, R>
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
        let pass_continue = self.state.1;
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

impl<B, C, S, P, L, R> CommandBuffer<B, C, ExecutableState<MultiShot<S>, P>, L, R>
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
        CommandBuffer<B, C, PendingState<ExecutableState<MultiShot<S>, P>>, L, R>,
    ) {
        let MultiShot(simultaneous) = self.state.0;
        let pass_continue = self.state.1;
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
