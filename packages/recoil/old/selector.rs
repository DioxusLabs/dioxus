use crate::Atom;

// =====================================
//    Selectors
// =====================================
pub struct SelectorApi {}
impl SelectorApi {
    pub fn get<T: PartialEq>(&self, t: &'static Atom<T>) -> &T {
        todo!()
    }
}
// pub struct SelectorBuilder<Out, const Built: bool> {
//     _p: std::marker::PhantomData<Out>,
// }
// impl<O> SelectorBuilder<O, false> {
//     pub fn getter(self, f: impl Fn(()) -> O) -> SelectorBuilder<O, true> {
//         todo!()
//         // std::rc::Rc::pin(value)
//         // todo!()
//     }
// }

pub struct selector<O>(pub fn(&SelectorApi) -> O);
// pub struct selector<O>(pub fn(SelectorBuilder<O, false>) -> SelectorBuilder<O, true>);
pub type Selector<O> = selector<O>;
