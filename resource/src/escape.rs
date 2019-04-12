//! Module provides wrapper for types that cannot be dropped silently.
//! Usually such types are required to be returned to their creator.
//! `Escape` wrapper help the user to do so by sending underlying value to the `Terminal` when it is dropped.
//! Users are encouraged to dispose of the values manually while `Escape` be just a safety net.

use {
    crossbeam_channel::{Receiver, Sender},
    std::{
        iter::repeat,
        mem::ManuallyDrop,
        ops::{Deref, DerefMut},
        ptr::{drop_in_place, read},
        sync::Arc,
    },
};

/// Allows values to "escape" dropping by sending them to the `Terminal`.
#[derive(Debug)]
pub struct Escape<T> {
    value: ManuallyDrop<T>,
    sender: Sender<T>,
}

impl<T> Escape<T> {
    /// Escape value.
    pub fn escape(value: T, terminal: &Terminal<T>) -> Self {
        Escape {
            value: ManuallyDrop::new(value),
            sender: Sender::clone(&terminal.sender),
        }
    }

    /// Unwrap escaping value.
    /// This will effectivly prevent it from escaping.
    pub fn unescape(escape: Self) -> T {
        unsafe {
            // Prevent `<Escape<T> as Drop>::drop` from being executed.
            let mut escape = ManuallyDrop::new(escape);

            // Release value from `ManuallyDrop`.
            let value = read(&mut *escape.value);

            // Drop sender. If it panics - value will be dropped.
            // Relevant values are allowed to be dropped due to panic.
            drop_in_place(&mut escape.sender);
            value
        }
    }

    /// Share escaped value.
    pub fn share(escape: Self) -> Handle<T> {
        escape.into()
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
        unsafe {
            // Read value from `ManuallyDrop` wrapper and send it over the channel.
            self.sender.send(read(&mut *self.value));
        }
    }
}

/// This types allows the user to create `Escape` wrappers.
/// Receives values from dropped `Escape` instances that was created by this `Terminal`.
#[derive(Debug)]
pub struct Terminal<T> {
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
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Terminal {
            sender: ManuallyDrop::new(sender),
            receiver,
        }
    }

    /// Wrap the value. It will be yielded by iterator returned by `Terminal::drain` if `Escape` will be dropped.
    pub fn escape(&self, value: T) -> Escape<T> {
        Escape::escape(value, &self)
    }

    // /// Check if `Escape` will send value to this `Terminal`.
    // pub fn owns(&self, escape: &Escape<T>) -> bool {
    //     *self.sender == escape.sender
    // }

    /// Get iterator over values from dropped `Escape` instances that was created by this `Terminal`.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        repeat(()).scan(&mut self.receiver, move |receiver, ()| {
            // trace!("Drain escape");
            if !receiver.is_empty() {
                receiver.recv()
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
            match self.receiver.try_recv() {
                None => {}
                Some(_) => {
                    log::error!("Terminal must be dropped after all `Escape`s");
                }
            }
        }
    }
}

/// Allows values to "escape" dropping by sending them to the `Terminal`.
/// Permit sharing unlike [`Escape`]
///
/// [`Escape`]: ./struct.Escape.html
#[derive(derivative::Derivative, Debug)]
#[derivative(Clone(bound = ""))]
pub struct Handle<T> {
    inner: Arc<Escape<T>>,
}

impl<T> From<Escape<T>> for Handle<T> {
    fn from(value: Escape<T>) -> Self {
        Handle {
            inner: Arc::new(value),
        }
    }
}

impl<T> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &**self.inner
    }
}
