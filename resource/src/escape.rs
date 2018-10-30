//! Module provides wrapper for types that cannot be dropped silently.
//! Usually such types are required to be returned to their creator.
//! `Escape` wrapper help the user to do so by sending underlying value to the `Terminal` when it is dropped.
//! Users are encouraged to dispose of the values manually while `Escape` be just a safety net.

use std::{
    iter::repeat,
    mem::{forget, ManuallyDrop},
    ops::{Deref, DerefMut},
    ptr::read,
};

use crossbeam_channel::{unbounded, Receiver, Sender};

/// Wraps value of any type and send it to the `Terminal` from which the wrapper was created.
/// In case `Terminal` is already dropped then value will be cast into oblivion via `std::mem::forget`.
#[derive(Debug, Clone)]
pub(crate) struct Escape<T> {
    value: ManuallyDrop<T>,
    sender: Sender<T>,
}

impl<T> Escape<T> {
    /// Unwrap the value.
    pub(crate) fn into_inner(escape: Self) -> T {
        Self::deconstruct(escape).0
    }

    fn deconstruct(mut escape: Self) -> (T, Sender<T>) {
        unsafe {
            let value = read(&mut *escape.value);
            let sender = read(&mut escape.sender);
            forget(escape);
            (value, sender)
        }
    }
}

impl<T> Deref for Escape<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.value
    }
}

impl<T> DerefMut for Escape<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.value
    }
}

impl<T> Drop for Escape<T> {
    fn drop(&mut self) {
        let value = unsafe { read(&mut *self.value) };
        self.sender.send(value)
    }
}

/// This types allows the user to create `Escape` wrappers.
/// Receives values from dropped `Escape` instances that was created by this `Terminal`.
#[derive(Debug)]
pub(crate) struct Terminal<T> {
    receiver: Receiver<T>,
    sender: ManuallyDrop<Sender<T>>,
}

impl<T> Default for Terminal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Terminal<T> {
    /// Create new `Terminal`.
    pub(crate) fn new() -> Self {
        let (sender, receiver) = unbounded();
        Terminal {
            sender: ManuallyDrop::new(sender),
            receiver,
        }
    }

    /// Wrap the value. It will be yielded by iterator returned by `Terminal::drain` if `Escape` will be dropped.
    pub(crate) fn escape(&self, value: T) -> Escape<T> {
        Escape {
            value: ManuallyDrop::new(value),
            sender: Sender::clone(&self.sender),
        }
    }

    // /// Check if `Escape` will send value to this `Terminal`.
    // pub(crate) fn owns(&self, escape: &Escape<T>) -> bool {
    //     *self.sender == escape.sender
    // }

    /// Get iterator over values from dropped `Escape` instances that was created by this `Terminal`.
    pub(crate) fn drain<'a>(&'a mut self) -> impl Iterator<Item = T> + 'a {
        repeat(()).scan((), move |&mut (), ()| {
            trace!("Drain escape");
            if !self.receiver.is_empty() {
                self.receiver.recv()
            } else {
                None
            }
        })
    }
}

impl<T> Drop for Terminal<T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.sender);
            match self.receiver.recv() {
                None => {}
                Some(_) => {
                    error!("Terminal must be dropped after all `Escape`s");
                }
            }
        }
    }
}
