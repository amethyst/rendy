// A dangerous bump allocator.

use std::marker::PhantomData;
use std::ptr::NonNull;
use std::ops::{Deref, DerefMut};
use std::alloc::{Layout, Allocator, AllocError};
use std::cell::Cell;

pub struct BumpInner {
    inner: bumpalo::Bump,
    references: Cell<usize>,
}

/// ## Safety
/// Even though this implements `Send`, sending some, but not all, references
/// to this allocator across threads and accessing them concurrently WILL result
/// in undefined behavior. This includes cloning in another thread.
pub struct Bump {
    inner: *mut BumpInner,
}
unsafe impl Sync for Bump {}

impl Drop for Bump {
    fn drop(&mut self) {
        let inner = unsafe { &*self.inner };
        inner.references.set(inner.references.get() - 1);

        if inner.references.get() == 0 {
            unsafe { Box::from_raw(self.inner) };
        }
    }
}

impl Clone for Bump {
    fn clone(&self) -> Self {
        let inner = unsafe { &*self.inner };
        inner.references.set(inner.references.get() + 1);
        Bump {
            inner: self.inner,
        }
    }
}

impl Bump {
    pub fn new() -> Self {
        let inner = BumpInner {
            inner: bumpalo::Bump::new(),
            references: Cell::new(1),
        };

        Self {
            inner: Box::into_raw(Box::new(inner)),
        }
    }

    pub fn reset(&mut self) {
        let inner = unsafe { &mut *self.inner };
        assert_eq!(inner.references.get(), 1);
        inner.inner.reset();
    }
}

unsafe impl Allocator for Bump {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let inner = unsafe { &*self.inner };
        Allocator::allocate(&&inner.inner, layout)
    }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let inner = unsafe { &*self.inner };
        Allocator::deallocate(&&inner.inner, ptr, layout)
    }
}
