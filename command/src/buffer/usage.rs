
/// Command buffer with this usage flag will move to invalid state after execution.
/// Resubmitting will require reset and rerecording commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct OneShot;

/// Command buffer with this usage flag will move back to executable state after execution.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiShot<S = NoSimultaneousUse>(pub S);

/// Additional flag that allows to resubmit buffer in pending state.
/// `Submit<B, SimultaneousUse>` can be submitted more than once.
#[derive(Clone, Copy, Debug, Default)]
pub struct SimultaneousUse;

/// Additional flag that allows to resubmit buffer in pending state.
/// `Submit<B, NoSimultaneousUse>` cannot be submitted more than once.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSimultaneousUse;

/// Buffers with this usage flag must be secondary buffers executed entirely in render-pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPassContinue;

/// Buffers with this usage flag are not render-pass continue buffers.
#[derive(Clone, Copy, Debug, Default)]
pub struct OutsideRenderPass;

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

impl Usage for NoSimultaneousUse {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::empty()
    }
}

impl Usage for RenderPassContinue {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::RENDER_PASS_CONTINUE
    }
}

impl Usage for OutsideRenderPass {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::empty()
    }
}
