use super::SecondaryLevel;

/// Command buffer with this usage flag will move to invalid state after execution.
/// Resubmitting will require reset and rerecording commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct OneShot;

/// Command buffer with this usage flag will move back to executable state after execution.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiShot<S = NoSimultaneousUse>(pub S);

/// Additional flag that allows resubmission of a command buffer while it is still in a pending state.
/// `Submit<B, SimultaneousUse>` can be submitted more than once.
#[derive(Clone, Copy, Debug, Default)]
pub struct SimultaneousUse;

/// Additional flag that disallows resubmission of a command buffer while it is still in a pending state
/// It must be completed, i.e. a fence must submitted with this buffer or later into the same queue
/// and be waited on before buffer resubmission.
/// `Submit<B, NoSimultaneousUse>` cannot be submitted more than once.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSimultaneousUse;

/// Buffers with this usage flag must be secondary buffers executed entirely in render-pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPassContinue;

/// Primary buffers must has this flag as they cannot has `RenderPassContinue` flag.
/// Secondary buffers with this usage flag cannot be executed as part of render-pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct OutsideRenderPass;

/// Type-level usage flags.
/// It defines if buffer can be resubmitted without reset.
/// Or even resubmitted while being executed.
pub trait Usage: Copy + Default + std::fmt::Debug + 'static {
    /// Flags required to begin command buffer.
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags;
}

impl Usage for OneShot {
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags {
        rendy_core::hal::command::CommandBufferFlags::ONE_TIME_SUBMIT
    }
}

impl Usage for MultiShot {
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags {
        rendy_core::hal::command::CommandBufferFlags::empty()
    }
}

impl Usage for MultiShot<SimultaneousUse> {
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags {
        rendy_core::hal::command::CommandBufferFlags::SIMULTANEOUS_USE
    }
}

impl Usage for NoSimultaneousUse {
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags {
        rendy_core::hal::command::CommandBufferFlags::empty()
    }
}

/// Trait implemented for type-level render pass relation flags.
/// `RenderPassContinue` and `OutsideRenderPass`.
pub trait RenderPassRelation<L>: Copy + Default + std::fmt::Debug + 'static {
    /// Flags required to begin command buffer.
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags;
}

impl RenderPassRelation<SecondaryLevel> for RenderPassContinue {
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags {
        rendy_core::hal::command::CommandBufferFlags::RENDER_PASS_CONTINUE
    }
}

impl<L> RenderPassRelation<L> for OutsideRenderPass {
    fn flags(&self) -> rendy_core::hal::command::CommandBufferFlags {
        rendy_core::hal::command::CommandBufferFlags::empty()
    }
}
