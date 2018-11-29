
mod command;

// use std::collections::VecDeque;
pub use self::command::{CommandCirque, CirqueEncoder, CirqueRenderPassInlineEncoder};

// struct Pending<T> {
//     value: T,
//     index: usize,
//     frame: u64,
// }

// struct Ready<T> {
//     value: T,
//     index: usize,
// }

// pub enum CirqueRef<'a, T, I = T, P = T> {
//     Initial(InitialRef<'a, T, I, P>),
//     Ready(ReadyRef<'a, T, I, P>),
// }

// pub struct InitialRef<'a, T, I = T, P = T> {
//     cirque: &'a mut Cirque<T, I, P>,
//     value: I,
// }

// impl<'a, T, I, P> InitialRef<'a, T, I, P> {
//     pub fn init(self, init: impl FnOnce(I) -> T) -> ReadyRef<'a, T, I, P> {
//         ReadyRef {
//             self.cirque,
//             value: init(self.value),
//         }
//     }
// }

// pub struct ReadyRef<'a, T, I = T, P = T> {
//     cirque: &'a mut Cirque<T, I, P>,
//     value: T,
// }

// impl<'a, T, I, P> ReadyRef<'a, T, I, P> {
//     pub fn finish(self, finish: impl FnOnce(T) -> P) {
//         self.cirque.pending.push_back(finish(self.value))
//     }
// }

// /// Resource ring buffer.
// pub struct Cirque<T, I = T, P = T> {
//     counter: usize,
//     pending: VecDeque<Pending<P>>,
//     ready: VecDeque<Ready<T>>,
// }

// impl<T, I, P> Cirque<T, I, P> {
//     pub unsafe fn get(
//         &mut self,
//         frames: std::ops::Range<u64>,

//     ) -> CirqueRef<'a, Self> {

//     }
// }

