use crate::handle::{HasValue, Handle};

struct DomainMarker;
pub type DomainHandle = Handle<DomainMarker>;
impl HasValue for DomainMarker {
    type Value = ManagedDomain;
}

pub struct ManagedDomain {
    inner: Arc<ManagedDomainInner>,
}

struct ManagedDomainInner {

}
