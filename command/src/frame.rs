//! Frame module docs.
use error::DeviceLost;

/// Unique index of the frame.
/// It must be unique per render instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameIndex(u64);

/// Generate `Frame`s.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
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
            index: self.next - 1,
        }
    }
}

/// Single frame rendering task.
/// Command buffers can be submitted as part of the `Frame`.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct Frame {
    index: u64,
}

impl Frame {
    /// Get frame index.
    pub fn index(&self) -> FrameIndex {
        FrameIndex(self.index)
    }
}

/// Frame that is fully submitted for execution.
/// User can wait for it to become `CompleteFrame`.
#[derive(Debug)]
pub struct PendingFrame<F> {
    index: FrameIndex,
    fences: Vec<F>,
}

impl<F> PendingFrame<F> {
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
    pub fn complete<D>(self, device: &D) -> Result<CompleteFrame<F>, Self> {
        if self.is_complete(device) {
            Ok(CompleteFrame {
                index: self.index,
                fences: self.fences,
            })
        } else {
            Err(self)
        }
    }

    /// Wait for the frame to complete and return `CompleteFrame` as a proof.
    pub fn wait<D>(self, device: &D) -> Result<CompleteFrame<F>, DeviceLost> {
        unimplemented!("Wait for the fences");
        Ok(CompleteFrame {
            index: self.index,
            fences: self.fences,
        })
    }
}

/// Proof that frame is complete.
#[derive(Debug)]
pub struct CompleteFrame<F> {
    index: FrameIndex,
    fences: Vec<F>,
}

impl<F> CompleteFrame<F> {
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
