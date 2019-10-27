//! Ring buffers for using with frames.

mod command;

pub use self::command::*;
use {
    crate::frame::{Frame, Frames},
    std::collections::VecDeque,
};

/// Reference to one of the values in the `Cirque`.
/// It can be in either initial or ready state.
#[derive(Debug)]
pub enum CirqueRef<'a, T, I = T, P = T> {
    /// Reference to value in initial state.
    Initial(InitialRef<'a, T, I, P>),

    /// Reference to value in ready state.
    Ready(ReadyRef<'a, T, I, P>),
}

impl<'a, T, I, P> CirqueRef<'a, T, I, P> {
    /// Init if not in ready state.
    pub fn or_init(self, init: impl FnOnce(I) -> T) -> ReadyRef<'a, T, I, P> {
        match self {
            CirqueRef::Initial(initial) => initial.init(init),
            CirqueRef::Ready(ready) => ready,
        }
    }

    /// Reset if not in initial state.
    pub fn or_reset(self, reset: impl FnOnce(T) -> I) -> InitialRef<'a, T, I, P> {
        match self {
            CirqueRef::Initial(initial) => initial,
            CirqueRef::Ready(ready) => ready.reset(reset),
        }
    }

    /// Get ref index.
    pub fn index(&self) -> usize {
        match self {
            CirqueRef::Initial(initial) => initial.index(),
            CirqueRef::Ready(ready) => ready.index(),
        }
    }
}

/// Reference to new value in the `Cirque`.
/// It is in initial state.
#[derive(Debug)]
pub struct InitialRef<'a, T, I = T, P = T> {
    relevant: relevant::Relevant,
    cirque: &'a mut Cirque<T, I, P>,
    value: I,
    frame: Frame,
    index: usize,
}

impl<'a, T, I, P> InitialRef<'a, T, I, P> {
    /// Init value.
    pub fn init(self, init: impl FnOnce(I) -> T) -> ReadyRef<'a, T, I, P> {
        ReadyRef {
            relevant: self.relevant,
            cirque: self.cirque,
            value: init(self.value),
            frame: self.frame,
            index: self.index,
        }
    }

    /// Get ref index.
    pub fn index(&self) -> usize {
        self.index
    }
}

/// Reference to value in the `Cirque`.
/// It is in ready state.
#[derive(Debug)]
pub struct ReadyRef<'a, T, I = T, P = T> {
    relevant: relevant::Relevant,
    cirque: &'a mut Cirque<T, I, P>,
    value: T,
    frame: Frame,
    index: usize,
}

impl<'a, T, I, P> ReadyRef<'a, T, I, P> {
    /// Init value.
    pub fn reset(self, reset: impl FnOnce(T) -> I) -> InitialRef<'a, T, I, P> {
        InitialRef {
            relevant: self.relevant,
            cirque: self.cirque,
            value: reset(self.value),
            frame: self.frame,
            index: self.index,
        }
    }

    /// Finish using this value.
    pub fn finish(self, finish: impl FnOnce(T) -> P) {
        self.relevant.dispose();
        self.cirque
            .pending
            .push_back((finish(self.value), self.index, self.frame))
    }

    /// Get ref index.
    pub fn index(&self) -> usize {
        self.index
    }
}

/// Resource cirque.
/// It simplifies using multiple resources
/// when same resource cannot be used simulteneously.
#[derive(Debug, derivative::Derivative)]
#[derivative(Default(bound = ""))]
pub struct Cirque<T, I = T, P = T> {
    pending: VecDeque<(P, usize, Frame)>,
    ready: VecDeque<(T, usize)>,
    marker: std::marker::PhantomData<fn() -> I>,
    counter: usize,
}

impl<T, I, P> Cirque<T, I, P> {
    /// Create new empty `Cirque`
    pub fn new() -> Self {
        Self::default()
    }

    /// Dispose of the `Cirque`.
    pub fn dispose(mut self, mut dispose: impl FnMut(either::Either<T, P>)) {
        self.pending
            .drain(..)
            .for_each(|(value, _, _)| dispose(either::Right(value)));
        self.ready
            .drain(..)
            .for_each(|(value, _)| dispose(either::Left(value)));
    }

    /// Get `CirqueRef` for specified frames range.
    /// Allocate new instance in initial state if no ready values exist.
    pub fn get<B: rendy_core::hal::Backend>(
        &mut self,
        frames: &Frames<B>,
        alloc: impl FnOnce() -> I,
        complete: impl Fn(P) -> T,
    ) -> CirqueRef<'_, T, I, P> {
        while let Some((value, index, frame)) = self.pending.pop_front() {
            if frames.is_complete(frame) {
                self.ready.push_back((complete(value), index));
            } else {
                self.pending.push_front((value, index, frame));
                break;
            }
        }
        if let Some((value, index)) = self.ready.pop_front() {
            CirqueRef::Ready(ReadyRef {
                relevant: relevant::Relevant,
                cirque: self,
                value,
                frame: frames.next(),
                index,
            })
        } else {
            self.counter += 1;
            let index = self.counter - 1;
            let value = alloc();
            CirqueRef::Initial(InitialRef {
                relevant: relevant::Relevant,
                index,
                cirque: self,
                value,
                frame: frames.next(),
            })
        }
    }
}

/// Resource cirque that depends on another one.
/// It relies on trusted ready index instead of frame indices.
/// It guarantees to always return same resource for same index.
#[derive(Debug, derivative::Derivative)]
#[derivative(Default(bound = ""))]
pub struct DependentCirque<T, I = T, P = T> {
    values: Vec<either::Either<T, P>>,
    marker: std::marker::PhantomData<fn() -> I>,
}
