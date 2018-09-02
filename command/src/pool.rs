
use relevant::Relevant;

use buffer::*;
use frame::Complete;
use device::Device;

/// Simple pool wrapper.
/// Doesn't provide any guarantees.
/// Wraps raw buffers into `Buffer`.
pub struct Pool<P> {
    raw: P,
    relevant: Relevant,
}

impl<P> Pool<P> {
    /// Allocate new buffer.
    fn allocate_buffers<D, L, R>(&mut self, device: &D, level: L, reset: R, count: usize) -> Vec<Buffer<D::CommandBuffer, InitialState, L, R>>
    where
        D: Device<CommandPool = P>,
    {
        unimplemented!()
    }

    /// Free buffers.
    /// Buffers must be in droppable state.
    fn free_buffers<D, L, S, R>(&mut self, device: &D, buffers: Vec<Buffer<D::CommandBuffer, S, L, R>>)
    where
        D: Device<CommandPool = P>,
        S: Droppable,
    {
        unimplemented!()
    }
}

pub struct FramePool<P, B> {
    raw: P,
    buffers: Vec<B>,
    relevant: Relevant,
}

impl<P, B> FramePool<P, B> {
    /// Reserve at least `count` buffers.
    pub fn reserve(&mut self, count: usize) {
        unimplemented!()
    }

    /// Bind pool to the particular frame.
    /// Command buffers acquired from it will be submitted as part of the frame.
    /// Once frame is complete and proof is acquired `FrameBoundPool::complete` can be used to convert pool back
    /// and resetting all commands buffers.
    pub fn bind<F>(self, frame: F) -> FrameBoundPool<P, B, F> {
        FrameBoundPool {
            raw: self.raw,
            buffers: self.buffers,
            frame,
            relevant: Relevant,
        }
    }
}

/// Command pool bound to particular frame.
pub struct FrameBoundPool<P, B, F> {
    raw: P,
    buffers: Vec<B>,
    frame: F,
    relevant: Relevant,
}

impl<P, B, F> FrameBoundPool<P, B, F> {
    /// Acquire command buffer from pool.
    /// The command buffer could be submitted only as part of submission for associated frame.
    pub fn acquire_buffer<'a, D, L>(&'a mut self, device: &D, level: L, count: usize) -> Buffer<FrameBuffer<'a, B, F>, InitialState, L>
    where
        D: Device<CommandBuffer = B>,
    {
        unimplemented!()
    }

    /// Complete frame using proof that is was complete acquire from signaled fence.
    ///
    /// # Panics
    ///
    /// This function will panic if complete proof associated with wrong frame.
    pub fn complete(self, proof: Complete<F>) -> FramePool<P, B> {
        unimplemented!()
    }
}
