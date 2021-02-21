use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Index;

use fxhash::FxHashMap;

use rendy_core::DeviceId;

use crate::Managed;
use super::{Handle, HandleGen, HasKey, HasValue};

pub struct EphemerialStore<M>
where
    M: HasKey + HasValue,
{
    gen: HandleGen<M>,
    forward: FxHashMap<M::Key, Handle<M>>,
    entries: FxHashMap<Handle<M>, Managed<M>>,
}

impl<M> EphemerialStore<M>
where
    M: HasKey + HasValue,
{

    pub fn new(device: DeviceId) -> Self {
        EphemerialStore {
            gen: HandleGen::new(device),
            forward: HashMap::default(),
            entries: HashMap::default(),
        }
    }

    pub fn lookup_key(&self, key: &M::Key) -> Option<Handle<M>> {
        self.forward.get(key).cloned()
    }

    pub fn insert(&mut self, key: M::Key, value: M::Value) -> Handle<M> {
        debug_assert!(&self.forward.contains_key(&key));

        let handle = self.gen.next();
        let managed = Managed::new(value, handle);
        self.forward.insert(key, handle);
        self.entries.insert(handle, managed);
        handle
    }

}

impl<M: HasKey + HasValue> Index<Handle<M>> for EphemerialStore<M> {
    type Output = Managed<M>;
    fn index(&self, idx: Handle<M>) -> &Managed<M> {
        unimplemented!()
    }
}
