use std::sync::atomic::{AtomicUsize, Ordering};

lazy_static::lazy_static! {
    static ref SHADER_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
}

/// Unique id for a shader.
///
/// For every call to `ShaderId::generate`, a new ID is returned, unique to the
/// current process.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ShaderId(usize);

impl ShaderId {

    /// Generates a new `ShaderId`. Will always return a unique id within a
    /// process.
    pub fn generate() -> Self {
        let id = SHADER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        ShaderId(id)
    }

    /// Gets the inner integer representation of the ID
    pub fn inner(&self) -> usize {
        self.0
    }

}
