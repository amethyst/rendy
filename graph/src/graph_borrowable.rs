use std::alloc::{Allocator, Global};
use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut};
use std::any::Any;

pub struct GraphBorrowable<T, A: Allocator + Clone = Global> {
    meta: *mut Meta,
    inner: *mut T,
    alloc: A,
}
impl<T, A: Allocator + Clone> Drop for GraphBorrowable<T, A> {
    fn drop(&mut self) {
        {
            let meta = unsafe { &*self.meta };
            assert!(
                !meta.borrowed_by_graph,
                "cannot drop GraphBorrowable while a borrow is active!"
            );
        }

        unsafe {
            Box::from_raw_in(self.meta, self.alloc.clone());
            Box::from_raw_in(self.inner, self.alloc.clone());
        }
    }
}

impl<T> GraphBorrowable<T, Global> {
    pub fn new(inner: T) -> Self {
        GraphBorrowable {
            meta: Box::into_raw(Box::new(Meta {
                borrowed_by_graph: false,
            })),
            inner: Box::into_raw(Box::new(inner)),
            alloc: Global,
        }
    }
}

impl<T, A: Allocator + Clone> GraphBorrowable<T, A> {
    pub fn new_in(inner: T, alloc: A) -> Self {
        GraphBorrowable {
            meta: Box::into_raw(Box::new_in(Meta {
                borrowed_by_graph: false,
            }, alloc.clone())),
            inner: Box::into_raw(Box::new_in(inner, alloc.clone())),
            alloc,
        }
    }

    pub fn take_borrow(&mut self) -> GraphBorrow<T> {
        {
            let meta = unsafe { &mut *self.meta };
            assert!(!meta.borrowed_by_graph);
            meta.borrowed_by_graph = true;
        }

        GraphBorrow {
            meta: self.meta,
            inner: self.inner,
        }
    }

    pub fn try_borrow(&self) -> Option<&T> {
        let meta = unsafe { &*self.meta };
        if meta.borrowed_by_graph {
            None
        } else {
            Some(unsafe { &*self.inner })
        }
    }

    pub fn try_borrow_mut(&mut self) -> Option<&mut T> {
        let meta = unsafe { &*self.meta };
        if meta.borrowed_by_graph {
            None
        } else {
            Some(unsafe { &mut *self.inner })
        }
    }

    #[track_caller]
    pub fn borrow_mut(&mut self) -> &mut T {
        self.try_borrow_mut().unwrap()
    }
}

impl<T, A: Allocator + Clone> Borrow<T> for GraphBorrowable<T, A> {
    #[track_caller]
    fn borrow(&self) -> &T {
        self.try_borrow().unwrap()
    }
}
impl<T, A: Allocator + Clone> BorrowMut<T> for GraphBorrowable<T, A> {
    #[track_caller]
    fn borrow_mut(&mut self) -> &mut T {
        self.try_borrow_mut().unwrap()
    }
}

pub struct GraphBorrow<T> {
    meta: *mut Meta,
    inner: *mut T,
}
impl<T> Drop for GraphBorrow<T> {
    fn drop(&mut self) {
        let meta = unsafe { &mut *self.meta };
        assert!(meta.borrowed_by_graph);
        meta.borrowed_by_graph = false;
    }
}

impl<T: 'static> GraphBorrow<T> {
    pub fn into_any(self) -> DynGraphBorrow {
        DynGraphBorrow {
            meta: self.meta,
            inner: self.inner,
        }
    }
}

impl<T> Deref for GraphBorrow<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner }
    }
}
impl<T> DerefMut for GraphBorrow<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner }
    }
}

pub struct DynGraphBorrow {
    meta: *mut Meta,
    inner: *mut dyn Any,
}
impl Drop for DynGraphBorrow {
    fn drop(&mut self) {
        let meta = unsafe { &mut *self.meta };
        assert!(meta.borrowed_by_graph);
        meta.borrowed_by_graph = false;
    }
}
impl Deref for DynGraphBorrow {
    type Target = dyn Any;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner }
    }
}
impl DerefMut for DynGraphBorrow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner }
    }
}

struct Meta {
    borrowed_by_graph: bool,
}
