use std::sync::Arc;
use std::hash::{Hash, Hasher};

use smallvec::SmallVec;

pub mod buffer;
pub mod image;
pub mod image_view;
pub mod shader_module;
pub mod sampler;
pub mod descriptor_set_layout;
pub mod pipeline_layout;
pub mod render_pass;
pub mod graphics_pipeline;
pub mod framebuffer;

use crate::{
    handle::{Handle, DynHandle, HasValue},
    dependent::Dependent,
};

pub struct Managed<T>
where
    T: HasValue,
{
    inner: Arc<ManagedInner<T>>,
}

impl<T> Managed<T>
where
    T: HasValue,
{

    pub(super) fn new(val: T::Value, handle: Handle<T>) -> Self {
        let inner = ManagedInner {
            value: val,
            handle,
            outgoing: SmallVec::new(),
            alive: true,
        };
        Managed {
            inner: Arc::new(inner),
        }
    }

}

impl<T> Clone for Managed<T>
where
    T: HasValue,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Hash for Managed<T>
where
    T: HasValue,
{
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.inner.handle.hash(hasher)
    }
}
impl<T> PartialEq for Managed<T>
where
    T: HasValue,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.handle == other.inner.handle
    }
}
impl<T> Eq for Managed<T> where T: HasValue {}

struct ManagedInner<T>
where
    T: HasValue,
{
    value: T::Value,

    handle: Handle<T>,

    outgoing: SmallVec<[Dependent; 4]>,

    /// If alive tracking is enabled, this is used to indicate that the inner
    /// value is usagle.
    /// This being false usually means that a resource it's derived from has
    /// been destroyed.
    alive: bool,
}

impl<T: HasValue> Managed<T> {

    pub fn handle(&self) -> Handle<T> {
        self.inner.handle
    }

}
