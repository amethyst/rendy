//! Pool module docs.

use std::fmt::Debug;

use relevant::Relevant;

use buffer::*;
use device::{CommandBuffer, Device};
use family::FamilyId;
use frame::{Frame, CompleteFrame};

/// Simple pool wrapper.
/// Doesn't provide any guarantees.
/// Wraps raw buffers into `Buffer`.
#[derive(Debug)]
pub struct Pool<P, C, R = ()> {
    inner: P,
    capability: C,
    reset: R,
    family: FamilyId,
    relevant: Relevant,
}

impl<P, C, R> Pool<P, C, R> {
    /// Allocate new buffer.
    fn allocate_buffers<D, L>(&mut self, device: &D, level: L, count: usize) -> Vec<Buffer<D::CommandBuffer, C, InitialState, L, R>>
    where
        D: Device<CommandPool = P>,
    {
        unimplemented!()
    }

    /// Free buffers.
    /// Buffers must be in droppable state.
    fn free_buffers<D, L, S>(&mut self, device: &D, buffers: Vec<Buffer<D::CommandBuffer, C, S, L, R>>)
    where
        D: Device<CommandPool = P>,
        S: Droppable,
    {
        unimplemented!()
    }

    /// Reset all buffers of this pool.
    pub unsafe fn reset(&mut self) {
        unimplemented!()
    }
}

/// Command pool that owns allocated buffers.
/// It can be used to borrow buffers one by one.
/// All buffers will be reset together via pool.
/// Prior reset user must ensure all buffers are complete.
#[derive(Debug)]
pub struct OwningPool<P, B, C, R = ()> {
    inner: Pool<P, C, R>,
    buffers: Vec<B>,
    next: usize,
}

impl<P, B, C, R> OwningPool<P, B, C, R> {
    /// Reserve at least `count` buffers.
    /// Allocate if there are not enough unused buffers.
    pub fn reserve(&mut self, count: usize) {
        unimplemented!()
    }

    /// Acquire command buffer from pool.
    /// The command buffer could be submitted only as part of submission for associated frame.
    /// TODO: Check that buffer cannot be moved out.
    pub fn acquire_buffer<D, L>(&mut self, device: &D, level: L) -> Buffer<&mut B, C, InitialState, L>
    where
        B: CommandBuffer + Debug,
        D: Device<CommandBuffer = B, Submit = B::Submit>,
    {
        unimplemented!()
    }

    /// Reset all buffers at once.
    ///
    /// # Safety
    ///
    /// All buffers from this pool must be in resettable state.
    /// Any primary buffer that references secondary buffer from this pool will be invalidated.
    pub unsafe fn reset(&mut self) {
        unimplemented!()
    }
}

/// `OwningPool` that can be bound to frame execution.
/// All command buffers acquired from bound `FramePool` are guarantee
/// to complete when frame's fence is set, and buffers can be reset.
#[derive(Debug)]
pub struct FramePool<P, B, C> {
    inner: OwningPool<P, B, C>,
}

impl<P, B, C> FramePool<P, B, C> {
    /// Bind pool to particular frame.
    /// Command pools acquired from the bound pool could be submitted only within frame borrowing lifetime.
    /// This ensures that frame's fences will be signaled after all commands from all command buffers from this pool
    /// are complete.
    pub fn bind<'a, F>(&'a mut self, frame: &'a Frame<F>) -> FrameBoundPool<'a, P, B, C, F> {
        FrameBoundPool {
            inner: &mut self.inner,
            frame,
        }
    }
}

/// `OwningPool` that bound to frame execution.
/// All command buffers acquired from bound `FrameBoundPool` are guarantee
/// to complete when frame's fence is set, and so buffers can be reset.
#[derive(Debug)]
pub struct FrameBoundPool<'a, P: 'a, B: 'a, C: 'a, F: 'a> {
    inner: &'a mut OwningPool<P, B, C>,
    frame: &'a Frame<F>,
}

impl<'a, P: 'a, B: 'a, C: 'a, F: 'a> FrameBoundPool<'a, P, B, C, F> {
    /// Reserve at least `count` buffers.
    /// Allocate if there are not enough unused buffers.
    pub fn reserve(&mut self, count: usize) {
        unimplemented!()
    }

    /// Acquire command buffer from pool.
    /// The command buffer could be submitted only as part of submission for associated frame.
    /// TODO: Check that buffer cannot be moved out.
    pub fn acquire_buffer<'b, D, L>(&'b mut self, device: &D, level: L) -> Buffer<FrameBoundBuffer<'b, 'a, B, F>, C, InitialState, L>
    where
        B: CommandBuffer + Debug,
        D: Device<CommandBuffer = B, Submit = B::Submit>,
    {
        unimplemented!()
    }

    /// Reset all buffers at once.
    ///
    /// # Safety
    ///
    /// All buffers from this pool must be in resettable state.
    /// Any primary buffer that references secondary buffer from this pool will be invalidated.
    pub fn reset(&mut self, complete: &CompleteFrame<F>) {
        unimplemented!()
    }
}


