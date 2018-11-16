//! Frame module docs.

use std::borrow::Borrow;

use command::{
    Capability, CommandBuffer, Encoder, ExecutableState, InitialState, MultiShot, OneShot,
    OwningCommandPool, PrimaryLevel, RecordingState, Resettable, Submit, Supports,
};

use factory::Factory;

/// Fences collection.
pub type Fences<B> = smallvec::SmallVec<[<B as gfx_hal::Backend>::Fence; 8]>;

/// Single frame rendering task.
/// Command buffers can be submitted as part of the `Frame`.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct Frame<B: gfx_hal::Backend> {
    index: u64,
    fences: Fences<B>,
}

impl<B> Frame<B>
where
    B: gfx_hal::Backend,
{
    /// Get frame index.
    pub fn index(&self) -> u64 {
        self.index
    }

    pub fn fences(&self) -> &[B::Fence] {
        &self.fences
    }
}

/// Proof that frame is complete.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct CompleteFrame {
    index: u64,
}

impl CompleteFrame {
    /// Get frame index.
    pub fn index(&self) -> u64 {
        self.index
    }
}

/// Timeline of frames, complete, pending and next.
#[derive(Debug)]
pub struct Frames<B: gfx_hal::Backend> {
    pending: std::collections::VecDeque<Fences<B>>,
    free: Vec<Fences<B>>,
    next: Frame<B>,
}

impl<B> Frames<B>
where
    B: gfx_hal::Backend,
{
    /// Create new `Frames` instance.
    pub fn new(factory: &mut Factory<B>, fences_count: usize) -> Self {
        Frames {
            pending: Default::default(),
            free: Default::default(),
            next: Frame {
                index: 0,
                fences: (0 .. fences_count).map(|_| factory.create_fence(false).unwrap()).collect(),
            }
        }
    }

    /// Get next frame reference.
    pub fn next(&self) -> &Frame<B> {
        &self.next
    }

    /// Advance to the next frame.
    /// All fences of the next frame must be queued.
    pub unsafe fn advance(&mut self, factory: &mut Factory<B>, fences_count: usize) -> &Frame<B> {
        let mut fences: Fences<B> = self.free.pop().unwrap_or(Fences::<B>::new());
        let add = (fences.len() .. fences_count).map(|_| factory.create_fence(false).unwrap());
        fences.extend(add);
        fences.truncate(fences_count);
        self.pending.push_back(std::mem::replace(&mut self.next.fences, fences));
        self.next.index += 1;
        &self.next
    }

    /// Get upper bound of complete frames.
    pub fn complete_upper_bound(&self) -> u64 {
        debug_assert!(self.pending.len() as u64 <= self.next.index);
        self.next.index - self.pending.len() as u64
    }

    /// Check if frame with specified index is complete.
    pub fn complete(&self, index: u64) -> Option<CompleteFrame> {
        if self.complete_upper_bound() > index {
            Some(CompleteFrame { index })
        } else {
            None
        }
    }

    /// Wait for completion of the frames until specified (inclusive)
    /// Returns proof.
    /// 
    /// # Parameters
    /// 
    /// `index` last index of frame that must complete.
    /// `factory` - The factory.
    /// 
    /// # Panics
    /// 
    /// This function will panic if `index` is greater than or equal to next frame.
    pub fn wait_complete(&mut self, target: u64, factory: &Factory<B>) -> CompleteFrame {
        assert!(target <= self.next.index());
        if let Some(complete) = self.complete(target) {
            complete
        } else {
            // n - p <= t
            // p - n + t + 1 >= 1
            // count >= 1
            let count = self.pending.len() - (self.next.index() - target - 1) as usize;
            let ready = factory.wait_for_fences(self.pending.iter().take(count).flatten(), gfx_hal::device::WaitFor::All, !0);
            assert_eq!(ready, Ok(true));
            self.free.extend(self.pending.drain( .. count).inspect(|fences| factory.reset_fences(fences.iter()).unwrap()));
            CompleteFrame {
                index: target,
            }
        }
    }
}
