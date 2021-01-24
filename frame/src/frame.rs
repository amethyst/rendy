//! Frame module docs.

use rendy_core::hal;

use crate::{command::Fence, factory::Factory};

/// Fences collection.
pub type Fences<B> = smallvec::SmallVec<[Fence<B>; 8]>;

/// Single frame rendering task.
/// Command buffers can be submitted as part of the `Frame`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_copy_implementations)]
pub struct Frame {
    index: u64,
}

impl Frame {
    /// Create frame with specific index.
    pub fn with_index(index: u64) -> Self {
        Frame { index }
    }

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

/// Complete - next frame range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramesRange {
    next: u64,
    complete_upper_bound: u64,
}

impl FramesRange {
    /// Check if given frame is.
    pub fn is_complete(&self, frame: Frame) -> bool {
        self.complete_upper_bound > frame.index
    }

    /// Check if given frame is.
    pub fn complete(&self, frame: Frame) -> Option<CompleteFrame> {
        if self.complete_upper_bound > frame.index {
            Some(CompleteFrame { index: frame.index })
        } else {
            None
        }
    }

    /// Get next frame
    pub fn next(&self) -> Frame {
        Frame { index: self.next }
    }
}

/// Timeline of frames, complete, pending and next.
#[derive(Debug)]
pub struct Frames<B: hal::Backend> {
    pending: std::collections::VecDeque<Fences<B>>,
    next: u64,
}

impl<B> Frames<B>
where
    B: hal::Backend,
{
    /// Create new `Frames` instance.
    pub fn new() -> Self {
        Frames {
            pending: Default::default(),
            next: 0,
        }
    }

    /// Get next frame reference.
    pub fn next(&self) -> Frame {
        Frame { index: self.next }
    }

    /// Advance to the next frame.
    /// All fences of the next frame must be queued.
    pub fn advance(&mut self, fences: Fences<B>) {
        self.pending.push_back(fences);
        self.next += 1;
    }

    /// Get upper bound of complete frames.
    /// All frames with index less than result of this function are complete.
    pub fn complete_upper_bound(&self) -> u64 {
        self.next - self.pending.len() as u64
    }

    /// Check if given frame is.
    pub fn is_complete(&self, frame: Frame) -> bool {
        self.complete_upper_bound() > frame.index
    }

    /// Check if frame with specified index is complete.
    pub fn complete(&self, frame: Frame) -> Option<CompleteFrame> {
        if self.complete_upper_bound() > frame.index {
            Some(CompleteFrame { index: frame.index })
        } else {
            None
        }
    }

    /// Wait for completion of the frames until specified (inclusive)
    /// Returns proof.
    ///
    /// # Parameters
    ///
    /// `target` - frame that must complete.
    /// `factory` - The factory.
    ///
    /// # Panics
    ///
    /// This function will panic if `target` is greater than or equal to next frame.
    pub fn wait_complete(
        &mut self,
        target: Frame,
        factory: &Factory<B>,
        free: impl FnMut(Fences<B>),
    ) -> CompleteFrame {
        if let Some(complete) = self.complete(target) {
            complete
        } else {
            // n - p <= t
            // p - n + t + 1 >= 1
            // count >= 1
            let count = self.pending.len() - (self.next - target.index - 1) as usize;
            factory
                .wait_for_fences(
                    self.pending.iter_mut().take(count).flatten(),
                    hal::device::WaitFor::All,
                    !0,
                )
                .unwrap();
            self.pending.drain(..count).for_each(free);
            CompleteFrame {
                index: target.index,
            }
        }
    }

    /// Dispose of the `Frames`
    pub fn dispose(mut self, factory: &mut Factory<B>) {
        factory
            .wait_for_fences(
                self.pending.iter_mut().flatten(),
                hal::device::WaitFor::All,
                !0,
            )
            .unwrap();

        self.pending
            .drain(..)
            .flatten()
            .for_each(|fence| factory.destroy_fence(fence));
    }

    /// Get range of frame indices in this form:
    /// `upper bound of finished frames .. next frame`.
    pub fn range(&self) -> FramesRange {
        FramesRange {
            next: self.next,
            complete_upper_bound: self.complete_upper_bound(),
        }
    }
}
