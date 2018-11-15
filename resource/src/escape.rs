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

#[derive(Debug)]
struct Inner<T> {
    value: ManuallyDrop<T>,
    sender: crossbeam_channel::Sender<T>,
}

impl<T> Inner<T> {
    /// Unwrap the value.

    fn into_inner(self) -> T {
        self.deconstruct().0
    }

    fn deconstruct(mut self) -> (T, crossbeam_channel::Sender<T>) {
        unsafe {
            let value = read(&mut *self.value);
            let sender = read(&mut self.sender);
            forget(self);
            (value, sender)
        }
    }
}

impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        let value = unsafe {
            // `self.value` cannot be accessed after this function.
            // `ManuallyDrop` will prevent `self.value` from dropping.
            read(&mut *self.value)
        };
        self.sender.send(value)
    }
}

/// Values of `KeepAlive` keeps resources from destroying.
/// 
/// # Example
/// 
/// ```no_run
/// # extern crate rendy_resource;
/// # use rendy_resource::*;
/// 
/// fn foo(buffer: Buffer<B>) {
///     let kp: KeepAlive = buffer.keep_alive();
/// 
///     // `kp` keeps this buffer from being destroyed.
///     // It still can be referenced by command buffer on used by GPU.
///     drop(buffer);
/// 
///     // If there is no `KeepAlive` instances created from this buffer
///     // then it can be destrouyed after this line.
///     drop(kp);
/// }
/// ```
#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct KeepAlive(#[derivative(Debug = "ignore")] std::sync::Arc<dyn std::any::Any + Send + Sync>);

/// Wraps value of any type and send it to the `Terminal` from which the wrapper was created.
/// In case `Terminal` is already dropped then value will be cast into oblivion via `std::mem::forget`.
#[derive(Debug)]
pub struct Escape<T> {
    access: *mut T,
    inner: std::sync::Arc<Inner<T>>,
}

impl<T> Escape<T> {
    pub fn keep_alive(escape: &Self) -> KeepAlive
    where
        T: Send + Sync + 'static,
    {
        KeepAlive(escape.inner.clone() as _)
    }

    /// Try to avoid channel sending if resource is not references elsewhere.
    pub fn dispose(escape: Self) -> Option<T> {
        std::sync::Arc::try_unwrap(escape.inner)
            .ok()
            .map(Inner::into_inner)
    }
}

impl<T> Deref for Escape<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            // Only `Escape` has access to `T`.
            // `KeepAlive` doesn't access `T`.
            // `Inner` only access `T` when dropped.
            &*self.access
        }
    }
}

impl<T> DerefMut for Escape<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            // Only `Escape` has access to `T`.
            // `KeepAlive` doesn't access `T`.
            // `Inner` only access `T` when dropped.
            &mut*self.access
        }
    }
}

/// This types allows the user to create `Escape` wrappers.
/// Receives values from dropped `Escape` instances that was created by this `Terminal`.
#[derive(Debug)]
pub struct Terminal<T: 'static> {
    receiver: crossbeam_channel::Receiver<T>,
    sender: ManuallyDrop<crossbeam_channel::Sender<T>>,
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
        let mut inner = std::sync::Arc::new(Inner {
            value: ManuallyDrop::new(value),
            sender: crossbeam_channel::Sender::clone(&self.sender),
        });

        // can't fail
        let access: *mut T = &mut *std::sync::Arc::get_mut(&mut inner).unwrap().value;

        Escape {
            access,
            inner,
        }
    }

    // /// Check if `Escape` will send value to this `Terminal`.
    // pub fn owns(&self, escape: &Escape<T>) -> bool {
    //     *self.sender == escape.sender
    // }

    /// Get iterator over values from dropped `Escape` instances that was created by this `Terminal`.
    pub fn drain<'a>(&'a mut self) -> impl Iterator<Item = T> + 'a {
        repeat(()).scan((), move |&mut (), ()| {
            // trace!("Drain escape");
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
