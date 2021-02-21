use std::ops::Index;
use std::collections::HashMap;

use fxhash::FxHashMap;

use rendy_core::DeviceId;

use crate::Managed;
use super::{Handle, HandleGen, HasValue};

pub struct InstanceStore<M>
where
    M: HasValue,
{
    gen: HandleGen<M>,
    entries: FxHashMap<Handle<M>, Managed<M>>,
}

impl<M> InstanceStore<M>
where
    M: HasValue,
{

    pub fn new(device: DeviceId) -> Self {
        InstanceStore {
            gen: HandleGen::new(device),
            entries: HashMap::default(),
        }
    }

    pub fn insert(&mut self, value: M::Value) -> Handle<M> {
        let handle = self.gen.next();
        let managed = Managed::new(value, handle);
        self.entries.insert(handle, managed);
        handle
    }

}

impl<M: HasValue> Index<Handle<M>> for InstanceStore<M> {
    type Output = Managed<M>;
    fn index(&self, idx: Handle<M>) -> &Managed<M> {
        unimplemented!()
    }
}
