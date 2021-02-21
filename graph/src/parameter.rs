use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct IdGenerator(usize);
impl IdGenerator {
    pub fn new() -> Self {
        Self(0)
    }
    pub fn next(&mut self) -> usize {
        let num = self.0;
        self.0 += 1;
        num
    }
}

#[derive(Default)]
pub struct ParameterStore {
    params: BTreeMap<DynamicParameter, Box<dyn Any>>,
}
impl ParameterStore {
    pub fn new() -> Self {
        Self {
            params: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.params.clear();
    }

    pub fn get<T: Any + 'static>(&self, param: Parameter<T>) -> Option<&T> {
        self.params.get(&param.into()).map(|b| b.downcast_ref::<T>().unwrap())
    }

    pub fn put<T: Any + 'static>(&mut self, param: Parameter<T>, value: T) -> bool {
        self.params.insert(param.into(), Box::new(value)).is_some()
    }
    pub fn put_dyn(&mut self, param: DynamicParameter, value: Box<dyn Any>) -> bool {
        debug_assert_eq!(param.1, value.type_id());
        self.params.insert(param, value).is_some()
    }
}

pub trait ParameterInput {
    type Input: Any;
    fn count() -> usize;
    fn collect(value: Self::Input, sink: &mut impl FnMut(Box<dyn Any>));
    fn walk(&self, walker: &mut impl FnMut(DynamicParameter));
    fn gen_params(gen: &mut IdGenerator) -> Self;
}

impl<T: 'static> ParameterInput for Parameter<T> {
    type Input = T;
    fn count() -> usize {
        1
    }
    fn collect(value: Self::Input, sink: &mut impl FnMut(Box<dyn Any>)) {
        sink(Box::new(value));
    }
    fn walk(&self, walker: &mut impl FnMut(DynamicParameter)) {
        walker((*self).into())
    }
    fn gen_params(gen: &mut IdGenerator) -> Self {
        Parameter(gen.next(), PhantomData)
    }
}
impl ParameterInput for () {
    type Input = ();
    fn count() -> usize {
        0
    }
    fn collect(value: Self::Input, sink: &mut impl FnMut(Box<dyn Any>)) {}
    fn walk(&self, walker: &mut impl FnMut(DynamicParameter)) {}
    fn gen_params(gen: &mut IdGenerator) -> Self {
        ()
    }
}
impl<A: ParameterInput> ParameterInput for (A,) {
    type Input = (A::Input, );
    fn count() -> usize {
        A::count()
    }
    fn collect((a, ): (A::Input, ), sink: &mut impl FnMut(Box<dyn Any>)) {
        A::collect(a, sink);
    }
    fn walk(&self, walker: &mut impl FnMut(DynamicParameter)) {
        self.0.walk(walker);
    }
    fn gen_params(gen: &mut IdGenerator) -> Self {
        (
            A::gen_params(gen),
        )
    }
}
impl<A: ParameterInput, B: ParameterInput> ParameterInput for (A, B) {
    type Input = (A::Input, B::Input);
    fn count() -> usize {
        A::count() + B::count()
    }
    fn collect((a, b): (A::Input, B::Input), sink: &mut impl FnMut(Box<dyn Any>)) {
        A::collect(a, sink);
        B::collect(b, sink);
    }
    fn walk(&self, walker: &mut impl FnMut(DynamicParameter)) {
        self.0.walk(walker);
        self.1.walk(walker);
    }
    fn gen_params(gen: &mut IdGenerator) -> Self {
        (
            A::gen_params(gen),
            B::gen_params(gen),
        )
    }
}
impl<A: ParameterInput, B: ParameterInput, C: ParameterInput> ParameterInput for (A, B, C) {
    type Input = (A::Input, B::Input, C::Input);
    fn count() -> usize {
        A::count() + B::count() + C::count()
    }
    fn collect((a, b, c): (A::Input, B::Input, C::Input), sink: &mut impl FnMut(Box<dyn Any>)) {
        A::collect(a, sink);
        B::collect(b, sink);
        C::collect(c, sink);
    }
    fn walk(&self, walker: &mut impl FnMut(DynamicParameter)) {
        self.0.walk(walker);
        self.1.walk(walker);
        self.2.walk(walker);
    }
    fn gen_params(gen: &mut IdGenerator) -> Self {
        (
            A::gen_params(gen),
            B::gen_params(gen),
            C::gen_params(gen),
        )
    }
}

#[derive(Debug)]
pub struct Parameter<T: 'static>(usize, PhantomData<T>);
impl<T: 'static> Copy for Parameter<T> {}
impl<T: 'static> Clone for Parameter<T> {
    fn clone(&self) -> Self {
        Parameter(self.0, PhantomData)
    }
}

/// A reified version of Parameter.
/// TODO derive eq and ord only from its index
/// This prevents the graph from being dependent on TypeId ord/eq
#[derive(Debug, Copy, Clone)]
pub struct DynamicParameter(usize, TypeId);
impl DynamicParameter {
    pub fn downcast<T: 'static>(self) -> Option<Parameter<T>> {
        if self.1 == TypeId::of::<T>() {
            Some(Parameter(self.0, PhantomData))
        } else {
            None
        }
    }
}
impl PartialEq<DynamicParameter> for DynamicParameter {
    fn eq(&self, rhs: &DynamicParameter) -> bool {
        if self.0 == rhs.0 {
            assert!(self.1 == rhs.1);
            true
        } else {
            false
        }
    }
}
impl Eq for DynamicParameter {}
impl Ord for DynamicParameter {
    fn cmp(&self, rhs: &DynamicParameter) -> std::cmp::Ordering {
        self.0.cmp(&rhs.0)
    }
}
impl PartialOrd<DynamicParameter> for DynamicParameter {
    fn partial_cmp(&self, rhs: &DynamicParameter) -> Option<std::cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}
impl<T: 'static> From<Parameter<T>> for DynamicParameter {
    fn from(other: Parameter<T>) -> Self {
        DynamicParameter(other.0, TypeId::of::<T>())
    }
}
