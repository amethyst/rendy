
//! Ring buffers for using with frames.

mod command;

use std::collections::VecDeque;
pub use self::command::*;

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
    pub fn or_init(self, init: impl FnOnce(I, usize) -> T) -> ReadyRef<'a, T, I, P> {
        match self {
            CirqueRef::Initial(initial) => initial.init(init),
            CirqueRef::Ready(ready) => ready,
        }
    }

    /// Reset if not in initial state.
    pub fn or_reset(self, reset: impl FnOnce(T, usize) -> I) -> InitialRef<'a, T, I, P> {
        match self {
            CirqueRef::Initial(initial) => initial,
            CirqueRef::Ready(ready) => ready.reset(reset),
        }
    }
}

/// Reference to new value in the `Cirque`.
/// It is in initial state.
#[derive(Debug)]
pub struct InitialRef<'a, T, I = T, P = T> {
    cirque: &'a mut Cirque<T, I, P>,
    value: I,
    frame: u64,
    index: usize,
}

impl<'a, T, I, P> InitialRef<'a, T, I, P> {
    /// Init value.
    pub fn init(self, init: impl FnOnce(I, usize) -> T) -> ReadyRef<'a, T, I, P> {
        ReadyRef {
            cirque: self.cirque,
            value: init(self.value, self.index),
            frame: self.frame,
            index: self.index,
        }
    }
}

/// Reference to value in the `Cirque`.
/// It is in ready state.
#[derive(Debug)]
pub struct ReadyRef<'a, T, I = T, P = T> {
    cirque: &'a mut Cirque<T, I, P>,
    value: T,
    frame: u64,
    index: usize,
}

impl<'a, T, I, P> ReadyRef<'a, T, I, P> {
    /// Init value.
    pub fn reset(self, reset: impl FnOnce(T, usize) -> I) -> InitialRef<'a, T, I, P> {
        InitialRef {
            cirque: self.cirque,
            value: reset(self.value, self.index),
            frame: self.frame,
            index: self.index,
        }
    }

    /// Finish using this value.
    pub fn finish(self, finish: impl FnOnce(T, usize) -> P) {
        self.cirque.pending.push_back((finish(self.value, self.index), self.index, self.frame))
    }
}

/// Resource cirque.
/// It simplifies using multiple resources
/// when same resource cannot be used simulteneously.
#[derive(Debug, derivative::Derivative)]
#[derivative(Default(bound = ""))]
pub struct Cirque<T, I = T, P = T> {
    pending: VecDeque<(P, usize, u64)>,
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
    pub fn dispose(
        mut self,
        mut dispose: impl FnMut(either::Either<T, P>, usize),
    ) {
        self.pending.drain(..).for_each(|(value, index, _)| dispose(either::Right(value), index));
        self.ready.drain(..).for_each(|(value, index)| dispose(either::Left(value), index));
    }

    /// Get `CirqueRef` for specified frames range.
    /// Allocate new instance in initial state if no ready values exist.
    pub fn get(
        &mut self,
        frames: std::ops::Range<u64>,
        alloc: impl FnOnce(usize) -> I,
        complete: impl Fn(P, usize) -> T,
    ) -> CirqueRef<'_, T, I, P> {
        while let Some((value, index, frame)) = self.pending.pop_front() {
            if frame > frames.start {
                self.pending.push_back((value, index, frame));
                break;
            }
            self.ready.push_back((complete(value, index), index));
        }
        if let Some((value, index)) = self.ready.pop_front() {
            CirqueRef::Ready(ReadyRef {
                cirque: self,
                value,
                frame: frames.end,
                index,
            })
        } else {
            self.counter += 1;
            let index = self.counter - 1;
            let value = alloc(index);
            CirqueRef::Initial(InitialRef {
                index,
                cirque: self,
                value,
                frame: frames.end,
            })
        }
    }
}
