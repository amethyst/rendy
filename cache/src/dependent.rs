use crate::handle::DynHandle;

pub struct ResourceProbe {}
pub struct ResourceProbeNotifier {}

pub struct ResourceProbeInner {}

pub enum Dependent {
    Handle(DynHandle),
    Probe(ResourceProbeNotifier),
}
