//! Frame module docs.

use ash::vk::Fence;
use error::DeviceLost;
use smallvec::SmallVec;

/// Fences collection.
pub type Fences = SmallVec<[Fence; 8]>;

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
    /// Only one `FrameGen` should be used.
    pub unsafe fn new() -> Self {
        FrameGen {
            next: 0,
        }
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
            Ok((
                CompleteFrame {
                    index: self.index,
                },
                self.fences,
            ))
        } else {
            Err(self)
        }
    }

    /// Wait for the frame to complete and return `CompleteFrame` as a proof.
    pub fn wait<D>(self, device: &D) -> Result<(CompleteFrame, Fences), DeviceLost> {
        unimplemented!("Wait for the fences");
        Ok((
            CompleteFrame {
                index: self.index,
            },
            self.fences,
        ))
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
    pub unsafe fn inner_ref(&self) -> &T {
        &self.value
    }

    /// Get mutable reference to bound value.
    ///
    /// # Safety
    ///
    /// Unbound value usage must not break frame-binding semantics.
    ///
    pub unsafe fn inner_mut(&mut self) -> &mut T {
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
