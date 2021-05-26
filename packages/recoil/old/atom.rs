use crate::{AtomValue, Readable, RecoilItem};

pub type Atom<T: PartialEq> = fn(&mut AtomBuilder<T>) -> T;

impl<T: AtomValue + 'static> Readable<T> for Atom<T> {
    fn load(&'static self) -> RecoilItem {
        todo!()
        // RecoilItem::Atom(self as *const _ as _)
    }
}

pub struct AtomBuilder<T: PartialEq> {
    pub key: String,
    _never: std::marker::PhantomData<T>,
}

impl<T: PartialEq> AtomBuilder<T> {
    pub fn new() -> Self {
        Self {
            key: "".to_string(),
            _never: std::marker::PhantomData {},
        }
    }

    pub fn set_key(&mut self, _key: &'static str) {}
}
