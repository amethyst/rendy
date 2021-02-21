use std::marker::PhantomData;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::any::{Any, TypeId};

use rendy_core::DeviceId;

mod instance_store;
pub use instance_store::InstanceStore;

mod ephemerial_store;
pub use ephemerial_store::EphemerialStore;

pub trait HasKey: HasValue {
    type Key: Clone + Eq + Hash + 'static;
}
pub trait HasValue: 'static {
    type Value: 'static;
}

pub struct HandleGen<T> {
    device: DeviceId,
    curr: usize,
    _phantom: PhantomData<T>,
}
impl<T> HandleGen<T> {

    pub fn new(device: DeviceId) -> Self {
        HandleGen {
            device,
            curr: 0,
            _phantom: PhantomData,
        }
    }

    pub fn next(&mut self) -> Handle<T> {
        self.curr += 1;
        Handle {
            device: self.device,
            idx: self.curr,
            _phantom: PhantomData,
        }
    }

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DynHandle {
    device: DeviceId,
    idx: usize,
    marker: TypeId,
}
impl DynHandle {

    pub fn try_cast<T: 'static>(&self) -> Option<Handle<T>> {
        let tid = TypeId::of::<T>();
        if tid == self.marker {
            Some(Handle {
                device: self.device,
                idx: self.idx,
                _phantom: PhantomData,
            })
        } else {
            None
        }
    }

}

pub struct Handle<T> {
    device: DeviceId,
    idx: usize,
    _phantom: PhantomData<T>,
}

impl<T> Handle<T> {

    pub fn device(&self) -> DeviceId {
        self.device
    }

}

impl<T> Debug for Handle<T> {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), std::fmt::Error> {
        let type_name = std::any::type_name::<T>();
        write!(fmt, "Handle({}, {})", type_name, self.idx)
    }
}
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            device: self.device,
            idx: self.idx,
            _phantom: PhantomData,
        }
    }
}
impl<T> Copy for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}
impl<T> Eq for Handle<T> {}
impl<T> Hash for Handle<T>
where
    T: Any,
{
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        let tid = TypeId::of::<T>();
        tid.hash(hasher);
        self.idx.hash(hasher);
    }
}
