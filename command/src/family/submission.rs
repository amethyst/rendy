use crate::buffer::{NoSimultaneousUse, OutsideRenderPass, PrimaryLevel, Submit, Submittable};
use rendy_core::hal;

#[allow(unused)]
type NoWaits<B> = std::iter::Empty<(
    &'static <B as hal::Backend>::Semaphore,
    hal::pso::PipelineStage,
)>;
#[allow(unused)]
type NoSignals<B> = std::iter::Empty<&'static <B as hal::Backend>::Semaphore>;
#[allow(unused)]
type NoSubmits<B> = std::iter::Empty<Submit<B, NoSimultaneousUse, PrimaryLevel, OutsideRenderPass>>;

/// Command queue submission.
#[derive(Debug)]
pub struct Submission<B, W = NoWaits<B>, C = NoSubmits<B>, S = NoSignals<B>> {
    /// Iterator over semaphores with stage flag to wait on.
    pub waits: W,

    /// Iterator over submittables.
    pub submits: C,

    /// Iterator over semaphores to signal.
    pub signals: S,

    /// Marker type for submission backend.
    pub marker: std::marker::PhantomData<fn() -> B>,
}

impl<B> Submission<B>
where
    B: hal::Backend,
{
    /// Create new empty submission.
    pub fn new() -> Self {
        Submission {
            waits: std::iter::empty(),
            submits: std::iter::empty(),
            signals: std::iter::empty(),
            marker: std::marker::PhantomData,
        }
    }
}

impl<B, W, S> Submission<B, W, NoSubmits<B>, S>
where
    B: hal::Backend,
{
    /// Add submits to the submission.
    pub fn submits<C>(self, submits: C) -> Submission<B, W, C, S>
    where
        C: IntoIterator,
        C::Item: Submittable<B>,
    {
        Submission {
            waits: self.waits,
            submits,
            signals: self.signals,
            marker: self.marker,
        }
    }
}

impl<B, C, S> Submission<B, NoWaits<B>, C, S>
where
    B: hal::Backend,
{
    /// Add waits to the submission.
    pub fn wait<'a, W, E>(self, waits: W) -> Submission<B, W, C, S>
    where
        W: IntoIterator<Item = (&'a E, hal::pso::PipelineStage)>,
        E: std::borrow::Borrow<B::Semaphore> + 'a,
    {
        Submission {
            waits,
            submits: self.submits,
            signals: self.signals,
            marker: self.marker,
        }
    }
}

impl<B, W, C> Submission<B, W, C, NoSignals<B>>
where
    B: hal::Backend,
{
    /// Add signals to the submission.
    pub fn signal<'a, S, E>(self, signals: S) -> Submission<B, W, C, S>
    where
        S: IntoIterator<Item = &'a E>,
        E: std::borrow::Borrow<B::Semaphore> + 'a,
    {
        Submission {
            waits: self.waits,
            submits: self.submits,
            signals,
            marker: self.marker,
        }
    }
}
