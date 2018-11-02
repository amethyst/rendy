//! Frame module docs.

use ash::{version::DeviceV1_0, vk};
use failure::Error;
use smallvec::SmallVec;
use std::borrow::Borrow;

use command::{
    Capability, CommandBuffer, Encoder, ExecutableState, InitialState, MultiShot, OneShot,
    OwningCommandPool, PrimaryLevel, RecordingState, Resettable, Submit,
};

/// Fences collection.
pub type Fences = SmallVec<[vk::Fence; 8]>;

/// Unique index of the frame.
/// It must be unique per render instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FrameIndex(u64);

/// Generate `Frame`s.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
#[repr(transparent)]
pub struct FrameGen {
    next: u64,
}

impl FrameGen {
    /// Create new `FrameGen`
    pub fn new() -> Self {
        FrameGen { next: 0 }
    }

    /// Generate next `Frame`.
    pub fn next(&mut self) -> Frame {
        self.next += 1;
        Frame {
            index: FrameIndex(self.next - 1),
        }
    }
}

/// Single frame rendering task.
/// Command buffers can be submitted as part of the `Frame`.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
#[repr(transparent)]
pub struct Frame {
    index: FrameIndex,
}

impl Frame {
    /// Get frame index.
    pub fn index(&self) -> FrameIndex {
        self.index
    }
}

/// Frame that is fully submitted for execution.
/// User can wait for it to become `CompleteFrame`.
#[derive(Debug)]
pub struct PendingFrame {
    index: FrameIndex,
    fences: Fences,
}

impl PendingFrame {
    /// Get frame index.
    pub fn index(&self) -> FrameIndex {
        self.index
    }

    /// Check if frame is complete on device.
    pub fn is_complete<D>(&self, device: &D) -> bool {
        unimplemented!("Check the fences")
    }

    /// Try to complete the frame.
    /// Returns `Ok(CompleteFrame {...})` if `is_complete` will return `true.
    /// Returns `Err(self)` otherwise.
    pub fn complete<D>(self, device: &D) -> Result<(CompleteFrame, Fences), Self> {
        if self.is_complete(device) {
            Ok((CompleteFrame { index: self.index }, self.fences))
        } else {
            Err(self)
        }
    }

    /// Wait for the frame to complete and return `CompleteFrame` as a proof.
    pub fn wait<D>(self, device: &D) -> Result<(CompleteFrame, Fences), Error> {
        unimplemented!("Wait for the fences");
        Ok((CompleteFrame { index: self.index }, self.fences))
    }
}

/// Proof that frame is complete.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct CompleteFrame {
    index: FrameIndex,
}

impl CompleteFrame {
    /// Get frame index.
    pub fn index(&self) -> FrameIndex {
        self.index
    }
}

/// Frame bound instance.
#[derive(Clone, Copy, Debug)]
pub struct FrameBound<'a, T> {
    frame: &'a Frame,
    value: T,
}

impl<'a, T> FrameBound<'a, T> {
    /// Bind value to frame
    pub fn bind(value: T, frame: &'a Frame) -> Self {
        FrameBound { frame, value }
    }

    /// Get reference to bound value.
    ///
    /// # Safety
    ///
    /// Unbound value usage must not break frame-binding semantics.
    ///
    pub unsafe fn value_ref(&self) -> &T {
        &self.value
    }

    /// Get mutable reference to bound value.
    ///
    /// # Safety
    ///
    /// Unbound value usage must not break frame-binding semantics.
    ///
    pub unsafe fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Unbind value from frame.
    ///
    /// # Safety
    ///
    /// Unbound value usage must not break frame-binding semantics.
    ///
    pub unsafe fn unbind(self) -> T {
        self.value
    }

    /// Get frame this value bound to.
    pub fn frame(&self) -> &'a Frame {
        self.frame
    }
}

/// Timeline of frames, complete, pending and next.
#[derive(Debug)]
pub struct Frames {
    pending: SmallVec<[PendingFrame; 5]>,
    next: Frame,
}

impl Frames {
    /// Get next frame reference.
    fn next(&self) -> &Frame {
        &self.next
    }

    /// Bind value to next frame.
    fn bind_to_next<T>(&self, value: T) -> FrameBound<'_, T> {
        FrameBound::bind(value, &self.next)
    }

    /// Get upper bound of complete frames.
    fn complete_until(&self) -> FrameIndex {
        self.pending.first().map_or(self.next.index, |p| p.index)
    }

    fn complete(&self, index: FrameIndex) -> Option<CompleteFrame> {
        if self.complete_until() > index {
            Some(CompleteFrame { index })
        } else {
            None
        }
    }
}

/// `OwningCommandPool` that can be bound to frame execution.
/// All command buffers acquired from bound `FramePool` are guarantee
/// to complete when frame's fence is set, and buffers can be reset.
#[derive(Debug)]
pub struct FramePool<C, R> {
    inner: OwningCommandPool<C, R>,
    frame: Option<FrameIndex>,
}

impl<C, R> FramePool<C, R> {
    /// Bind pool to particular frame.
    ///
    /// Command pools acquired from the bound pool could be submitted only within frame borrowing lifetime.
    /// This ensures that frame's fences will be signaled after all commands from all command buffers from this pool
    /// are complete.
    ///
    /// `reset` method must be called with `CompleteFrame` created from the bound `Frame` before binding to the another `Frame`.
    ///
    /// # Panics
    ///
    /// This function will panic if pool is still bound to frame.
    ///
    pub fn bind<'a, F>(&'a mut self, frame: &'a Frame) -> FrameBound<'a, &'a mut Self> {
        assert!(
            self.frame.is_none(),
            "`FramePool::reset` must be called before binding to another frame"
        );

        self.frame = Some(frame.index());

        FrameBound::bind(self, frame)
    }

    /// Reset all buffers at once.
    ///
    /// # Panics
    ///
    /// This function will panic if pool wasn't bound to the specified frame.
    ///
    pub fn reset(&mut self, complete: &CompleteFrame) {
        assert_eq!(
            self.frame.take(),
            Some(complete.index()),
            "CommandPool must be bound to the specified frame"
        );
        unimplemented!()
    }
}

impl<R> FramePool<vk::QueueFlags, R> {
    /// Convert capability level
    pub fn from_flags<C>(self) -> Result<FramePool<C, R>, Self>
    where
        C: Capability,
    {
        match self.inner.from_flags::<C>() {
            Ok(inner) => Ok(FramePool {
                inner,
                frame: self.frame,
            }),
            Err(inner) => Err(FramePool {
                inner,
                frame: self.frame,
            }),
        }
    }
}

impl<'a, C: 'a, R: 'a> FrameBound<'a, &'a mut FramePool<C, R>> {
    /// Reserve at least `count` buffers.
    /// Allocate if there are not enough unused buffers.
    pub fn reserve(&mut self, count: usize) {
        unimplemented!()
    }

    /// Acquire command buffer from pool.
    /// The command buffer could be submitted only as part of submission for associated frame.
    /// TODO: Check that buffer cannot be moved out.
    pub fn acquire_buffer<D, L>(
        &mut self,
        device: &impl DeviceV1_0,
        level: L,
    ) -> FrameBound<'a, CommandBuffer<C, InitialState, L>> {
        unimplemented!()
    }
}

impl<'a, S, L, C> FrameBound<'a, CommandBuffer<C, S, L>>
where
    S: Resettable,
{
    /// Release borrowed buffer. This allows to acquire next buffer from pool.
    /// Whatever state this buffer was in it will be reset only after bounded frame is complete.
    /// This allows safely to release borrowed buffer in pending state.
    pub fn release(self) {
        unimplemented!()
    }
}

impl<'a, C, R> FrameBound<'a, CommandBuffer<C, ExecutableState<OneShot>, PrimaryLevel, R>> {
    /// Produce `Submit` object that can be used to populate submission.
    pub fn submit(self) -> (FrameBound<'a, Submit>,) {
        unimplemented!()
    }
}

impl<'a, C, U, L, R> Encoder<C> for FrameBound<'a, CommandBuffer<C, RecordingState<U>, L, R>> {
    unsafe fn raw(&mut self) -> vk::CommandBuffer {
        CommandBuffer::raw(&self.value)
    }
}
