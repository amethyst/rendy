use {
    crate::escape::{Escape, Handle, Terminal},
    smallvec::SmallVec,
    std::collections::VecDeque,
};

/// Resource usage epochs.
#[derive(Clone, Debug)]
pub struct Epochs {
    pub values: SmallVec<[SmallVec<[u64; 8]>; 4]>,
}

impl Epochs {
    fn is_before(&self, other: &Self) -> bool {
        debug_assert_eq!(self.values.len(), other.values.len());
        self.values.iter().zip(other.values.iter()).all(|(a, b)| {
            debug_assert_eq!(a.len(), b.len());
            a.iter().zip(b.iter()).all(|(a, b)| a < b)
        })
    }
}

/// Resource handler.
#[derive(Debug, derivative::Derivative)]
#[derivative(Default(bound = ""))]
pub struct Resources<T> {
    terminal: Terminal<T>,
    dropped: VecDeque<(Epochs, T)>,
}

impl<T> Resources<T> {
    /// Create new resource manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap resource instance into handle.
    pub fn escape(&self, resource: T) -> Escape<T>
    where
        T: Sized,
    {
        Escape::escape(resource, &self.terminal)
    }

    /// Wrap resource instance into handle.
    pub fn handle(&self, resource: T) -> Handle<T>
    where
        T: Sized,
    {
        self.escape(resource).into()
    }

    /// Cleanup dropped resources.
    ///
    /// # Safety
    ///
    /// `next` epochs must contain epoch indices that aren't started yet
    /// `complete` epochs must contain epoch indices that are complete.
    /// Can be guaranteed with fence wait.
    ///
    pub fn cleanup(&mut self, mut dispose: impl FnMut(T), next: &Epochs, complete: &Epochs) {
        while let Some((epoch, resource)) = self.dropped.pop_front() {
            if !epoch.is_before(complete) {
                self.dropped.push_front((epoch, resource));
                break;
            }

            dispose(resource);
        }

        self.dropped
            .extend(self.terminal.drain().map(|res| (next.clone(), res)));
    }

    /// Cleanup all dropped resources.
    ///
    /// # Safety
    ///
    /// All dropped resources must be unused.
    /// Can be guaranteed with device idle wait.
    ///
    pub fn dispose(&mut self, dispose: impl FnMut(T)) {
        self.dropped
            .drain(..)
            .map(|(_, res)| res)
            .chain(self.terminal.drain())
            .for_each(dispose);
    }
}
