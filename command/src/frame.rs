//! Frame module docs.

use error::DeviceLost;

/// Unique index of the frame.
/// It must be unique per render instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameIndex(u64);

/// Single frame rendering task.
/// Command buffers can be submitted as part of the `Frame`.
/// Internally frame is just an index and fences.
/// But semantically it owns list of submissions submitted through it.
#[derive(Debug)]
pub struct Frame<F> {
    index: FrameIndex,
    fences: Vec<F>,
}

impl<F> Frame<F> {
    /// Create new frame instance.
    /// 
    /// # Safety
    /// 
    /// Index must be unique.
    pub unsafe fn new(index: FrameIndex) -> Self {
        Frame {
            index,
            fences: Vec::new(),
        }
    }

    /// Get frame index.
    pub fn index(&self) -> FrameIndex {
        self.index
    }

    /// Takes slice of fences associated with this frame.
    /// 
    pub unsafe fn fences(&self) -> &[F] {
        &self.fences
    }

    /// Finish frame.
    /// Returned `PendingFrame` can be used to wait the frame to complete on device.
    pub fn finish(self) -> PendingFrame<F> {
        PendingFrame {
            index: self.index,
            fences: self.fences,
        }
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

