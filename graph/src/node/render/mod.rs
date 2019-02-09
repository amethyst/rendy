//!
//! Advanced render pass node.
//! Will replace render pass node when polished.
//!

mod group;
mod pass;

pub use self::{group::*, pass::*};

/// Result of draw preparation.
#[derive(Clone, Copy, Debug)]
#[must_use]
pub enum PrepareResult {
    /// Force record draw commands.
    DrawRecord,

    /// Reuse draw commands.
    DrawReuse,
}

impl PrepareResult {
    fn force_record(&self) -> bool {
        match self {
            PrepareResult::DrawRecord => true,
            PrepareResult::DrawReuse => false,
        }
    }
}
