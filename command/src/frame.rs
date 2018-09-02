

/// Index of the frame.
pub struct FrameIndex(u64);

/// Single frame rendering task.
/// Command buffers can be submitted as part of the `Frame`.
pub struct Frame<F> {
    index: FrameIndex,
    fence: F,
}


impl<F> Frame<F> {
    pub fn submit()
}

/// Proof that frame is complete.
pub struct Complete<F> {
    raw: F,
}

