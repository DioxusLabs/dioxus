use super::Select;

pub type Atom<V> = fn() -> V;
pub type Selector<V> = fn(Select) -> V;

pub trait UniqueIds {
    type Id;
    fn static_id(self) -> Self::Id;
}

impl<V> UniqueIds for Atom<V> {
    type Id = AtomId;
    fn static_id(self) -> AtomId {
        AtomId::new(self)
    }
}

impl<V> UniqueIds for Selector<V> {
    type Id = SelectorId;
    fn static_id(self) -> SelectorId {
        SelectorId::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtomId(pub *const ());
impl AtomId {
    fn new<V>(atom: Atom<V>) -> Self {
        Self(std::ptr::addr_of!(atom) as *const ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SelectorId(pub *const ());
impl SelectorId {
    fn new<V>(atom: Selector<V>) -> Self {
        Self(std::ptr::addr_of!(atom) as *const ())
    }
}
