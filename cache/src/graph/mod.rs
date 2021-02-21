use fxhash::FxHashSet;

use crate::handle::DynHandle;

struct Graph {
    nodes: FxHashMap<DynHandle, FxHashSet<DynHandle>>,
}
