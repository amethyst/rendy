//! This module provides wrapper for types that cannot be dropped silently.
//! Usually such types are required to be returned to their creator,
//! for example many Vulkan resources must be destroyed by the same
//! Vulkan instance that created them.  Because they need some outside
//! context to be destroyed, Rust's `Drop` trait alone cannot handle them.
//! The `Escape` wrapper helps the user handle these values by sending the
//! underlying value to a `Terminal` when it is dropped.  The user can
//! then remove those values from the `Terminal` elsewhere in the program
//! and destroy them however necessary.
//! Users are encouraged to dispose of values manually while using `Escape`
//! as just a safety net.

use {
    crossbeam_channel::{Receiver, Sender},
    std::{
        iter::repeat,
        mem::ManuallyDrop,
        ptr::{drop_in_place, read},
        sync::Arc,
    },
};

use std::ops::Deref as StdDeref;

use derive_more::{Deref, DerefMut};

/// Allows values to "escape" dropping by sending them to the `Terminal`.
#[derive(Debug, Deref, DerefMut)]
pub struct Escape<T> {
    #[deref(forward)]
    #[deref_mut(forward)]
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
            let value = read(&*escape.value);

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

impl<T> Drop for Escape<T> {
    fn drop(&mut self) {
        unsafe { self.sender.send(read(&*self.value)).unwrap_or(()) }
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
            if !receiver.is_empty() {
                receiver.recv().ok()
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
            let _ = self.receiver.try_recv();
        }
    }
}

/// Allows values to "escape" dropping by sending them to the `Terminal`.
/// Permit sharing unlike [`Escape`]
///
/// [`Escape`]: ./struct.Escape.html
#[derive(Debug)]
pub struct Handle<T> {
    inner: Arc<Escape<T>>,
}

impl<T> StdDeref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &**self.inner
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            inner: self.inner.clone(),
        }
    }
}

impl<T> From<Escape<T>> for Handle<T> {
    fn from(value: Escape<T>) -> Self {
        Handle {
            inner: Arc::new(value),
        }
    }
}
