//! Module provides wrapper for types that cannot be dropped silently.
//! Usually such types are required to be returned to their creator.
//! `Escape` wrapper help the user to do so by sending underlying value to the `Terminal` when it is dropped.
//! Users are encouraged to dispose of the values manually while `Escape` be just a safety net.

use {
    std::{
        sync::Arc,
        cell::UnsafeCell,
        iter::repeat,
        mem::{forget, ManuallyDrop},
        ops::{Deref, DerefMut},
        ptr::read,
    },
    crossbeam_channel::{Receiver, Sender},
};

#[derive(Debug)] // `Debug` impl doesn't access value stored in `UnsafeCell`
struct Inner<T> {
    sender: Sender<T>,
    value: UnsafeCell<Option<T>>,
}

impl<T> Inner<T> {
    /// This function must be called at most once for given instance.
    /// No other access to the inner value is possible until `Any`.
    unsafe fn escape(&self, value: T) {
        debug_assert!((*self.value.get()).is_none());
        *self.value.get() = Some(value);
    }
}

unsafe impl<T: Send> Send for Inner<T> {}
unsafe impl<T: Send + Sync> Sync for Inner<T> {}

impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        if let Some(value) = unsafe {&mut*self.value.get()}.take() {
            self.sender.send(value);
        }
    }
}

/// Allows values to "escape" dropping by sending them to the `Terminal`.
#[derive(Debug)]
pub struct Escape<T> {
    value: ManuallyDrop<T>,
    inner: Arc<Inner<T>>,
}

impl<T> Escape<T> {
    /// Create new `Escape` bound to given `Terminal`.
    pub fn new(value: T, terminal: &Terminal<T>) -> Self {
        Escape {
            value: ManuallyDrop::new(value),
            inner: Arc::new(Inner {
                sender: Sender::clone(&terminal.sender),
                value: UnsafeCell::new(None),
            }),
        }
    }

    /// Keep escaped value alive until all `KeepAlive` instanced created from it are dropped.
    pub fn keep_alive(&self) -> KeepAlive
    where
        T: Send + Sync + 'static,
    {
        KeepAlive(self.inner.clone() as _)
    }

    /// Unwrap escaping value.
    /// This will effectivly prevent it from escaping.
    /// In case of existing `KeepAlive` created from this instance this function will **escape** value instead.
    pub fn unescape(mut self) -> Option<T> {
        unsafe {
            let inner = read(&mut self.inner);
            let value = read(&mut *self.value);
            forget(self);

            match Arc::try_unwrap(inner) {
                Ok(_) => Some(value),
                Err(inner) => { inner.escape(value); None }
            }
        }
    }
}

impl<T> Drop for Escape<T> {
    fn drop(&mut self) {
        unsafe {
            self.inner.escape(read(&mut *self.value));
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

/// Allows values to "escape" dropping by sending them to the `Terminal`.
/// Unlike `Escape` it doesn't support `KeepAlive` mechanism but has less overhead in exchange.
#[derive(Debug)]
pub struct EscapeCheap<T> {
    value: ManuallyDrop<T>,
    sender: Sender<T>,
}

impl<T> EscapeCheap<T> {
    /// Create new `Escape` bound to given `Terminal`.
    pub fn new(value: T, terminal: &Terminal<T>) -> Self {
        EscapeCheap {
            sender: Sender::clone(&terminal.sender),
            value: ManuallyDrop::new(value),
        }
    }
}

impl<T> Deref for EscapeCheap<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.value
    }
}

impl<T> DerefMut for EscapeCheap<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.value
    }
}

impl<T> Drop for EscapeCheap<T> {
    fn drop(&mut self) {
        unsafe {
            self.sender.send(read(&mut *self.value));
        }
    }
}

/// Allows values to "escape" dropping by sending them to the `Terminal`.
/// Unlike `Escape` it doesn't allow mutable access, but permits sharing via `Clone`.
#[derive(Debug, derivative::Derivative)]
#[derivative(Clone(bound = ""))]
pub struct EscapeShared<T> {
    inner: Arc<EscapeCheap<T>>,
}

impl<T> EscapeShared<T> {
    /// Create new `Escape` bound to given `Terminal`.
    pub fn new(value: T, terminal: &Terminal<T>) -> Self {
        EscapeShared {
            inner: Arc::new(EscapeCheap::new(value, terminal)),
        }
    }

    /// Keep escaped value alive until all `KeepAlive` instanced created from it are dropped.
    pub fn keep_alive(&self) -> KeepAlive
    where
        T: Send + Sync + 'static,
    {
        KeepAlive(self.inner.clone() as _)
    }

    /// Unwrap escaping value.
    /// This will effectivly prevent it from escaping.
    /// In case of existing `KeepAlive` created from this instance this function will **escape** value instead.
    pub fn unescape(self) -> Option<T> {
        match Arc::try_unwrap(self.inner) {
            Ok(mut inner) => unsafe {
                let value = read(&mut *inner.value);
                drop(read(&mut inner.sender));
                forget(inner);
                Some(value)
            },
            Err(_) => None,
        }
    }
}

impl<T> Deref for EscapeShared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.inner.value
    }
}

/// Values of `KeepAlive` keeps resources from destroying.
/// 
/// # Example
/// 
/// ```
/// # extern crate rendy_resource;
/// # use rendy_resource::*;
/// 
/// fn foo<B: gfx_hal::Backend>(buffer: Buffer<B>) {
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
#[derive(Clone, Debug)]
pub struct KeepAlive(std::sync::Arc<dyn std::any::Any + Send + Sync>);

/// This types allows the user to create `Escape` wrappers.
/// Receives values from dropped `Escape` instances that was created by this `Terminal`.
#[derive(Debug)]
pub struct Terminal<T: 'static> {
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
        Escape::new(value, &self)
    }

    /// Wrap the value. It will be yielded by iterator returned by `Terminal::drain` if `EscapeShared` will be dropped.
    pub fn escape_shared(&self, value: T) -> EscapeShared<T> {
        EscapeShared::new(value, &self)
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
            match self.receiver.recv() {
                None => {}
                Some(_) => {
                    error!("Terminal must be dropped after all `Escape`s");
                }
            }
        }
    }
}
