
/// Command buffer with this usage flag will move to invalid state after execution.
/// Resubmitting will require reset and rerecording commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct OneShot;

/// Command buffer with this usage flag will move back to executable state after execution.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiShot<S = ()>(pub S);

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

impl Usage for () {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::empty()
    }
}

impl Usage for RenderPassContinue {
    fn flags(&self) -> gfx_hal::command::CommandBufferFlags {
        gfx_hal::command::CommandBufferFlags::RENDER_PASS_CONTINUE
    }
}
