
use {
    interact::{Function, Access, MutAccess, ImmutAccess, ReflectMut, Reflect, ReflectDirect, Reflector, NodeTree, Climber, ClimbError, NodeInfo}},
    gfx_hal::Backend,
    crate::factory::Factory,
};

impl<B> Access for Factory<B>
where
    B: Backend,
{
    fn immut_access(&self) -> ImmutAccess {
        ImmutAccess {
            reflect: Reflect::Direct(self),
            functions: &[Function { name: "memory_utilization()", args: &[] }],
        }
    }
    fn mut_access(&mut self) -> MutAccess {
        MutAccess {
            reflect: ReflectMut::Immutable,
            functions: &[],
        }
    }
}

impl<B> ReflectDirect for Factory<B>
where
    B: Backend,
{
    fn immut_reflector(&self, reflector: &Arc<Reflector>) -> NodeTree {
        let obj_ptr = ((self as *const _) as usize, 0);
        let meta = match Reflector::seen_ptr(reflector, obj_ptr) {
            Ok(v) => return v,
            Err(meta) => meta,
        };
        NodeInfo::Leaf(Cow::Owned(format!("{:#?}", self))).with_meta(meta)
    }

    fn immut_climber<'a>(
        &self,
        _climber: &mut Climber<'a>,
    ) -> Result<Option<NodeTree>, ClimbError> {
        Ok(None)
    }

    fn mut_climber<'a>(
        &mut self,
        _climber: &mut Climber<'a>,
    ) -> Result<Option<NodeTree>, ClimbError> {
        Ok(None)
    }
}
