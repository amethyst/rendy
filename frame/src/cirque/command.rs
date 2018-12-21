
use {
    crate::command::{
        CommandBuffer, CommandPool,
        InitialState, ExecutableState, PendingState,
        MultiShot, NoSimultaneousUse, OutsideRenderPass, PrimaryLevel,
        Submit, Encoder, Level, Capability, IndividualReset,
    },
    super::*,
};

/// 
pub type CommandCirque<B, C, P = OutsideRenderPass, L = PrimaryLevel> = Cirque<
    CommandBuffer<B, C, ExecutableState<MultiShot, P>, L, IndividualReset>,
    CommandBuffer<B, C, InitialState, L, IndividualReset>,
    CommandBuffer<B, C, PendingState<ExecutableState<MultiShot, P>>, L, IndividualReset>,
>;

impl<B, C, L> CommandCirque<B, C, OutsideRenderPass, L>
where
    B: gfx_hal::Backend,
    L: Level,
    C: Capability,
{
    /// Encode and submit.
    pub fn encode_submit(
        &mut self,
        frames: std::ops::Range<u64>,
        force: bool,
        pool: &mut CommandPool<B, C, IndividualReset>,
        encode: impl FnOnce(Encoder<'_, B, C, L>, usize)
    ) -> Submit<B, NoSimultaneousUse, L, OutsideRenderPass> {
        let init = |initial: CommandBuffer<_, _, InitialState, _, _>, index| -> CommandBuffer<_, _, ExecutableState<MultiShot>, _, _> {
            let mut recording = initial.begin();
            encode(recording.encoder(), index);
            recording.finish()
        };

        let cr = self.get(
            frames,
            |_| pool.allocate_buffers(1).pop().unwrap(),
            |pending, _| unsafe {
                pending.mark_complete()
            },
        );

        let ready = if force {
            cr.or_init(init)
        } else {
            cr.or_reset(|executable, _| executable.reset())
                .init(init)
        };

        let mut slot = None;

        ready.finish(|executable, _| {
            let (submit, pending) = executable.submit();
            slot = Some(submit);
            pending
        });

        slot.unwrap()
    }
}