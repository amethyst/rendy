use std::collections::VecDeque;

use crate::{
    command::{
        Capability, CommandBuffer, CommandPool, ExecutableState, IndividualReset, RecordingState, Submit,
        InitialState, Level, MultiShot, PendingState, Usage, PrimaryLevel,
    },
    frame::Frames,
};

#[derive(Debug)]
pub struct CommandCirque<B: gfx_hal::Backend, C, P = (), S = (), L = PrimaryLevel> {
    pool: CommandPool<B, C, IndividualReset>,
    pendings: VecDeque<Pending<B, C, P, S, L>>,
    executables: VecDeque<Executable<B, C, P, S, L>>,
    level: L,
    counter: usize,
}

#[derive(Debug)]
struct Pending<B: gfx_hal::Backend, C, P, S, L> {
    buffer: CommandBuffer<B, C, PendingState<ExecutableState<MultiShot<P, S>>>, L, IndividualReset>,
    index: usize,
    frame_index: u64,
}

#[derive(Debug)]
struct Executable<B: gfx_hal::Backend, C, P, S, L> {
    buffer: CommandBuffer<B, C, ExecutableState<MultiShot<P, S>>, L, IndividualReset>,
    index: usize,
}

impl<B, C, P, S, L> CommandCirque<B, C, P, S, L>
where
    B: gfx_hal::Backend,
    C: Capability,
    L: Level,
{
    /// Create new command cirque for pool.
    pub fn new(pool: CommandPool<B, C, IndividualReset>, level: L) -> Self {
        CommandCirque {
            pool,
            pendings: VecDeque::new(),
            executables: VecDeque::new(),
            level,
            counter: 0,
        }
    }

    /// Get executable buffer from this cirque.
    ///
    /// # Safety
    ///
    /// This function must be called for the same `Frames` instance.
    pub unsafe fn get<'a, 'b>(
        &'a mut self,
        frames: &'b Frames<B>,
    ) -> either::Either<
        CirqueEncoder<'a, 'b, B, C, P, S, L, ExecutableState<MultiShot<P, S>>>,
        CirqueEncoder<'a, 'b, B, C, P, S, L, InitialState>,
    > {
        let upper = frames.complete_upper_bound();
        while self
            .pendings
            .front()
            .as_ref()
            .map_or(false, |pending| pending.frame_index < upper)
        {
            let pending = self.pendings.pop_front().unwrap();
            // All commands from this buffer are complete.
            let buffer = pending.buffer.complete();
            self.executables.push_back(Executable {
                buffer,
                index: pending.index,
            });
        }

        if let Some(executable) = self.executables.pop_front() {
            either::Left(CirqueEncoder {
                buffer: executable.buffer,
                index: executable.index,
                cirque: self,
                frames,
            })
        } else {
            let buffer = self.pool.allocate_buffers(self.level, 1).remove(0);
            self.counter += 1;

            either::Right(CirqueEncoder {
                buffer,
                index: self.counter,
                cirque: self,
                frames,
            })
        }
    }
}

/// Buffer borrowed from `CommandCirque`.
/// It is bound to `Frames` reference lifetime to ensure it can't be used with another frame.
#[derive(Debug)]
pub struct CirqueEncoder<'a, 'b, B: gfx_hal::Backend, C: 'a, P: 'a = (), S: 'a = (), L: 'a = PrimaryLevel, X = MultiShot<P, S>> {
    buffer: CommandBuffer<B, C, X, L, IndividualReset>,
    frames: &'b Frames<B>,
    cirque: &'a mut CommandCirque<B, C, P, S, L>,
    index: usize,
}

impl<'a, 'b, B, C, P, S, L> CirqueEncoder<'a, 'b, B, C, P, S, L, InitialState>
where
    B: gfx_hal::Backend,
{
    /// Begin recording command buffer.
    pub fn begin(self) -> CirqueEncoder<'a, 'b, B, C, P, S, L, RecordingState<MultiShot<P, S>>>
    where
        MultiShot<P, S>: Usage,
    {
        CirqueEncoder {
            buffer: self.buffer.begin(Default::default()),
            frames: self.frames,
            cirque: self.cirque,
            index: self.index,
        }
    }
}

impl<'a, 'b, B, C, P, S, L> CirqueEncoder<'a, 'b, B, C, P, S, L, RecordingState<MultiShot<P, S>>>
where
    B: gfx_hal::Backend,
{
    /// Begin render pass encoding.
    pub unsafe fn begin_render_pass_inline(
        &mut self,
        render_pass: &B::RenderPass, 
        framebuffer: &B::Framebuffer, 
        render_area: gfx_hal::pso::Rect, 
        clear_values: &[gfx_hal::command::ClearValueRaw],
    ) -> CirqueRenderPassInlineEncoder<'_, B> {
        let buffer = self.buffer.raw();
        unsafe {
            gfx_hal::command::RawCommandBuffer::begin_render_pass(
                buffer,
                render_pass,
                framebuffer,
                render_area,
                clear_values,
                gfx_hal::command::SubpassContents::Inline,
            );

            // `CirqueRenderPassInlineEncoder` allows to record only render pass commands.
            CirqueRenderPassInlineEncoder {
                buffer,
                index: self.index,
            }
        }
    }

    /// Finish recording command buffer.
    pub fn finish(self) -> CirqueEncoder<'a, 'b, B, C, P, S, L, ExecutableState<MultiShot<P, S>>>
    where
        MultiShot<P, S>: Usage,
    {
        CirqueEncoder {
            buffer: self.buffer.finish(),
            frames: self.frames,
            cirque: self.cirque,
            index: self.index,
        }
    }
}

impl<'a, 'b, B, C, P, S, L> CirqueEncoder<'a, 'b, B, C, P, S, L, ExecutableState<MultiShot<P, S>>>
where
    B: gfx_hal::Backend,
    P: Copy,
    S: Copy,
    L: Copy,
{
    /// Reset command buffer.
    pub fn reset(self) -> CirqueEncoder<'a, 'b, B, C, P, S, L, InitialState> {
        CirqueEncoder {
            buffer: self.buffer.reset(),
            frames: self.frames,
            cirque: self.cirque,
            index: self.index,
        }
    }

    /// Submit commands.
    /// This function creates submit instance bound to `Frames` reference lifetime.
    /// This guarantees that it will be submitted during current frame.
    /// Or not submitted at all.
    ///
    /// Command buffer is returned to `CommandCirque` as pending with frame index attached.
    /// Once frame is complete command buffer can be reused.
    pub fn submit(self) -> Submit<'b, B, P, S, L> {
        let (submit, buffer) = self.buffer.submit();
        self.cirque.pendings.push_back(Pending {
            buffer,
            index: self.index,
            frame_index: self.frames.next().index(),
        });

        submit
    }
}

#[derive(Debug)]
pub struct CirqueRenderPassInlineEncoder<'a, B: gfx_hal::Backend> {
    buffer: &'a mut B::CommandBuffer,
    index: usize,
}

impl<'a, B> CirqueRenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{

}

impl<'a, B> Drop for CirqueRenderPassInlineEncoder<'a, B>
where
    B: gfx_hal::Backend,
{
    fn drop(&mut self) {
        unsafe {
            gfx_hal::command::RawCommandBuffer::end_render_pass(
                self.buffer
            )
        }
    }
}

