use std::collections::HashSet;

use crate::{AtomBuilder, AtomRoot, Select};

/// All Atoms are `Readable` - they support reading their value.
///
/// This trait lets Dioxus abstract over Atoms, AtomFamilies, AtomRefs, and Selectors.
/// It is not very useful for your own code, but could be used to build new Atom primitives.
pub trait Readable<V, B = ()> {
    fn read(&self, root: AtomRoot) -> Option<V>;
    fn init(&self) -> V;
    fn unique_id(&self) -> AtomId;
}

/// All Atoms are `Writable` - they support writing their value.
///
/// This trait lets Dioxus abstract over Atoms, AtomFamilies, AtomRefs, and Selectors.
/// This trait lets Dioxus abstract over Atoms, AtomFamilies, AtomRefs, and Selectors
pub trait Writable<V, B = ()>: Readable<V, B> {
    fn write(&self, root: AtomRoot, value: V);
}

pub type Atom<V> = fn(AtomBuilder) -> V;
pub type Selector<V> = fn(Select) -> V;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtomId(pub *const ());
impl AtomId {
    pub fn new<V>(atom: Atom<V>) -> Self {
        Self(std::ptr::addr_of!(atom) as *const ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SelectorId(pub *const ());
impl SelectorId {
    pub fn new<V>(atom: Selector<V>) -> Self {
        Self(std::ptr::addr_of!(atom) as *const ())
    }
}

pub struct Selection {
    pub id: SelectorId,
    pub deps: HashSet<Dep>,
    pub val: *mut (),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dep {
    Atom(AtomId),
    Selector(SelectorId),
}
