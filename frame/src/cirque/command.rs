use {
    super::*,
    crate::command::{
        Capability, CommandBuffer, CommandPool, ExecutableState, IndividualReset, InitialState,
        Level, MultiShot, NoSimultaneousUse, OutsideRenderPass, PendingState, PrimaryLevel,
        RenderPassRelation, Submit,
    },
};

///
pub type CommandCirque<B, C, P = OutsideRenderPass, L = PrimaryLevel> = Cirque<
    CommandBuffer<B, C, ExecutableState<MultiShot, P>, L, IndividualReset>,
    CommandBuffer<B, C, InitialState, L, IndividualReset>,
    CommandBuffer<B, C, PendingState<ExecutableState<MultiShot, P>>, L, IndividualReset>,
>;

///
pub type CommandCirqueRef<'a, B, C, P = OutsideRenderPass, L = PrimaryLevel> = CirqueRef<
    'a,
    CommandBuffer<B, C, ExecutableState<MultiShot, P>, L, IndividualReset>,
    CommandBuffer<B, C, InitialState, L, IndividualReset>,
    CommandBuffer<B, C, PendingState<ExecutableState<MultiShot, P>>, L, IndividualReset>,
>;

///
pub type CommandInitialRef<'a, B, C, P = OutsideRenderPass, L = PrimaryLevel> = InitialRef<
    'a,
    CommandBuffer<B, C, ExecutableState<MultiShot, P>, L, IndividualReset>,
    CommandBuffer<B, C, InitialState, L, IndividualReset>,
    CommandBuffer<B, C, PendingState<ExecutableState<MultiShot, P>>, L, IndividualReset>,
>;

///
pub type CommandReadyRef<'a, B, C, P = OutsideRenderPass, L = PrimaryLevel> = ReadyRef<
    'a,
    CommandBuffer<B, C, ExecutableState<MultiShot, P>, L, IndividualReset>,
    CommandBuffer<B, C, InitialState, L, IndividualReset>,
    CommandBuffer<B, C, PendingState<ExecutableState<MultiShot, P>>, L, IndividualReset>,
>;

impl<B, C, P, L> CommandCirque<B, C, P, L>
where
    B: rendy_core::hal::Backend,
    L: Level,
    C: Capability,
    P: RenderPassRelation<L>,
{
    /// Encode and submit.
    pub fn encode<'a>(
        &'a mut self,
        frames: &Frames<B>,
        pool: &mut CommandPool<B, C, IndividualReset>,
        encode: impl FnOnce(CommandCirqueRef<'a, B, C, P, L>) -> CommandReadyRef<'a, B, C, P, L>,
    ) -> Submit<B, NoSimultaneousUse, L, P> {
        let cr = self.get(
            frames,
            || pool.allocate_buffers(1).pop().unwrap(),
            |pending| unsafe { pending.mark_complete() },
        );

        let ready = encode(cr);

        let mut slot = None;

        ready.finish(|executable| {
            let (submit, pending) = executable.submit();
            slot = Some(submit);
            pending
        });

        slot.unwrap()
    }
}
