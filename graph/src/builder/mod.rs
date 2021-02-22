use std::marker::PhantomData;

use crate::scheduler::{
    builder::ProceduralBuilder,
    interface::GraphCtx,
};
use crate::factory::Factory;
use crate::core::hal::Backend;
use crate::core::hal::window::PresentationSurface;
use super::parameter::{ParameterInput, IdGenerator, ParameterStore, Parameter, DynamicParameter};

mod context;
pub use self::context::{GraphConstructCtx, PassConstructCtx, StandaloneConstructCtx};

pub trait Node<B: Backend>: 'static {
    type Result: ParameterInput;
    fn construct(
        &mut self,
        factory: &mut Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        store: &ParameterStore,
    ) -> Result<<Self::Result as ParameterInput>::Input, ()>;
}

pub struct SimpleNode<F, R> {
    fun: F,
    phantom: PhantomData<R>,
}
impl<B, R, F> Node<B> for SimpleNode<F, R>
where
    B: Backend,
    R: ParameterInput + 'static,
    F: FnMut(&mut Factory<B>, &mut GraphConstructCtx<B>, &ParameterStore) -> Result<R::Input, ()> + 'static,
{
    type Result = R;
    fn construct(
        &mut self,
        factory: &mut Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        store: &ParameterStore,
    ) -> Result<<Self::Result as ParameterInput>::Input, ()> {
        (self.fun)(factory, ctx, store)
    }
}

trait NodeDyn<B: Backend> {
    fn construct(
        &mut self,
        factory: &mut Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        parameters: &[DynamicParameter],
        store: &mut ParameterStore,
    ) -> Result<(), ()>;
}
impl<B: Backend, T: Node<B>> NodeDyn<B> for T {
    fn construct(
        &mut self,
        factory: &mut Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        parameters: &[DynamicParameter],
        store: &mut ParameterStore,
    ) -> Result<(), ()> {
        let ret = Node::construct(self, factory, ctx, store)?;

        let mut params_iter = parameters.iter();
        T::Result::collect(ret, &mut |val| {
            let param = params_iter.next().unwrap();
            debug_assert!(!store.put_dyn(*param, val));
        });

        Ok(())
    }
}

pub struct GfxSchedulerTypes<B: Backend>(PhantomData<B>);
impl<B: Backend> crate::scheduler::SchedulerTypes for GfxSchedulerTypes<B> {
    type Image = GraphImage<B>;
    type Buffer = B::Buffer;
    type Semaphore = B::Semaphore;
}

pub enum GraphImage<B: Backend> {
    Image(B::Image),
    SwapchainImage(<B::Surface as PresentationSurface<B>>::SwapchainImage),
}

pub struct GraphBuilder<B: Backend> {
    phantom: PhantomData<B>,

    gen: IdGenerator,
    nodes: Vec<Box<dyn NodeDyn<B>>>,

    store: ParameterStore,
    inner: ProceduralBuilder<GfxSchedulerTypes<B>>,
}

impl<B: Backend> GraphBuilder<B> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,

            gen: IdGenerator::new(),
            nodes: Vec::new(),

            store: ParameterStore::new(),
            inner: ProceduralBuilder::new(),
        }
    }

    pub fn add<N: Node<B>>(&mut self, node: N) -> Result<N::Result, ()> {
        self.nodes.push(Box::new(node));
        let result = N::Result::gen_params(&mut self.gen);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{GraphBuilder, ProceduralBuilder, ParameterStore, NodeExecution, Node};

    pub struct Abc;
    impl Node for Abc {
        type Result = ();
        fn construct(
            &mut self,
            ctx: &mut ProceduralBuilder,
            store: &ParameterStore,
        ) -> Result<((), NodeExecution), ()> {
            todo!()
        }
    }

    #[test]
    fn asbasd() {
        let mut builder = GraphBuilder::new();

        builder.add(Abc).unwrap();

    }

}
