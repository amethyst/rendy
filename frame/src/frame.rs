//! Frame module docs.

use crate::{command::Fence, factory::Factory};

/// Fences collection.
pub type Fences<B> = smallvec::SmallVec<[Fence<B>; 8]>;

/// Single frame rendering task.
/// Command buffers can be submitted as part of the `Frame`.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Frame {
    index: u64,
}

impl Frame {
    /// Get frame index.
    pub fn index(&self) -> u64 {
        self.index
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
    next: Frame,
}

impl<B> Frames<B>
where
    B: gfx_hal::Backend,
{
    /// Create new `Frames` instance.
    pub fn new() -> Self {
        Frames {
            pending: Default::default(),
            next: Frame { index: 0 },
        }
    }

    /// Get next frame reference.
    pub fn next(&self) -> &Frame {
        &self.next
    }

    /// Advance to the next frame.
    /// All fences of the next frame must be queued.
    pub unsafe fn advance(&mut self, fences: Fences<B>) {
        self.pending.push_back(fences);
        self.next.index += 1;
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
    /// `target` - last index of frame that must complete.
    /// `factory` - The factory.
    ///
    /// # Panics
    ///
    /// This function will panic if `target` is greater than or equal to next frame.
    pub fn wait_complete(
        &mut self,
        target: u64,
        factory: &Factory<B>,
        free: impl FnMut(Fences<B>),
    ) -> CompleteFrame {
        assert!(target <= self.next.index());
        if let Some(complete) = self.complete(target) {
            complete
        } else {
            // n - p <= t
            // p - n + t + 1 >= 1
            // count >= 1
            let count = self.pending.len() - (self.next.index() - target - 1) as usize;
            let ready = factory.wait_for_fences(
                self.pending.iter_mut().take(count).flatten(),
                gfx_hal::device::WaitFor::All,
                !0,
            );
            assert_eq!(ready, Ok(true));
            self.pending.drain(..count).for_each(free);
            CompleteFrame { index: target }
        }
    }

    /// Dispose of the `Frames`
    pub fn dispose(mut self, factory: &mut Factory<B>) {
        let ready = factory.wait_for_fences(
            self.pending.iter_mut().flatten(),
            gfx_hal::device::WaitFor::All,
            !0,
        );
        assert_eq!(ready, Ok(true));

        self.pending
            .drain(..)
            .flatten()
            .for_each(|fence| factory.destroy_fence(fence));
    }

    /// Get range of frame indices in this form:
    /// `upper bound of finished frames .. next frame`.
    pub fn range(&self) -> std::ops::Range<u64> {
        self.complete_upper_bound()..self.next.index
    }
}
