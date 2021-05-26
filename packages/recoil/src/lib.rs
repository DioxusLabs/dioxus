use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    rc::Rc,
};

pub trait FamilyKey: PartialEq + Hash {}
impl<T: PartialEq + Hash> FamilyKey for T {}

pub trait AtomValue: PartialEq + Clone {}
impl<T: PartialEq + Clone> AtomValue for T {}

// Atoms, selectors, and their family variants are readable
pub trait Readable<T> {}

// Only atoms and atom families are writable
// Selectors are not
pub trait Writeable<T>: Readable<T> {}

// =================
//    Atoms
// =================

pub struct AtomBuilder {}
pub type Atom<T> = fn(&mut AtomBuilder) -> T;
impl<T> Readable<T> for Atom<T> {}
impl<T> Writeable<T> for Atom<T> {}

pub type AtomFamily<K, V, F = HashMap<K, V>> = fn((K, V)) -> F;

pub trait SelectionSelector<K, V> {
    fn select(&self, k: &K) -> CollectionSelection<V> {
        todo!()
    }
}
impl<K, V, F> SelectionSelector<K, V> for AtomFamily<K, V, F> {}

pub trait FamilyCollection<K, V> {}
impl<K, V> FamilyCollection<K, V> for HashMap<K, V> {}
pub struct CollectionSelection<T> {
    _never: PhantomData<T>,
}
impl<T> Readable<T> for CollectionSelection<T> {}

// =================
//    Selectors
// =================
pub struct SelectorBuilder {}
impl SelectorBuilder {
    pub fn get<T: PartialEq>(&self, t: &impl Readable<T>) -> &T {
        todo!()
    }
}
pub type Selector<T> = fn(&mut SelectorBuilder) -> T;
impl<T> Readable<T> for Selector<T> {}

pub struct SelectorFamilyBuilder {}

impl SelectorFamilyBuilder {
    pub fn get<T: PartialEq>(&self, t: &impl Readable<T>) -> &T {
        todo!()
    }
}

/// Create a new value as a result of a combination of previous values
/// If you need to return borrowed data, check out [`SelectorFamilyBorrowed`]
pub type SelectorFamily<Key, Value> = fn(&mut SelectorFamilyBuilder, Key) -> Value;

impl<K, V> Readable<V> for SelectorFamily<K, V> {}

/// Borrowed selector families are – surprisingly - discouraged.
/// This is because it's not possible safely memoize these values without keeping old versions around.
///
/// However, it does come in handy to borrow the contents of an item without re-rendering child components.
pub type SelectorFamilyBorrowed<Key, Value> =
    for<'a> fn(&'a mut SelectorFamilyBuilder, Key) -> &'a Value;

impl<'a, K, V: 'a> SelectionSelector<K, V> for fn(&'a mut SelectorFamilyBuilder, K) -> V {}
// =================
//    API
// =================
pub struct RecoilApi {}
impl RecoilApi {
    pub fn get<T: PartialEq>(&self, t: &'static Atom<T>) -> Rc<T> {
        todo!()
    }
    pub fn modify<T: PartialEq, O>(&self, t: &'static Atom<T>, f: impl FnOnce(&mut T) -> O) -> O {
        todo!()
    }
    pub fn set<T: PartialEq>(&self, t: &'static Atom<T>, new: T) {
        self.modify(t, move |old| *old = new);
    }
}

// ================
//    Root
// ================
type AtomId = u32;
type ConsumerId = u32;

pub struct RecoilRoot {
    consumers: RefCell<HashMap<AtomId, Box<dyn RecoilSlot>>>,
}

trait RecoilSlot {}

impl RecoilRoot {
    pub(crate) fn new() -> Self {
        Self {
            consumers: Default::default(),
        }
    }
}

pub use hooks::*;
mod hooks {
    use super::*;
    use dioxus_core::prelude::Context;

    pub fn use_init_recoil_root(ctx: Context) {
        ctx.use_create_context(move || RecoilRoot::new())
    }

    pub fn use_set_state<'a, T: PartialEq>(
        c: Context<'a>,
        t: &'static impl Writeable<T>,
    ) -> &'a Rc<dyn Fn(T)> {
        todo!()
    }

    pub fn use_recoil_state<'a, T: PartialEq + 'static>(
        ctx: Context<'a>,
        readable: &'static impl Writeable<T>,
    ) -> (&'a T, &'a Rc<dyn Fn(T)>) {
        todo!()
    }

    pub fn use_recoil_value<'a, T: PartialEq>(
        ctx: Context<'a>,
        t: &'static impl Readable<T>,
    ) -> &'a T {
        todo!()
    }

    pub fn use_recoil_callback<'a, F: 'a>(
        ctx: Context<'a>,
        f: impl Fn(RecoilApi) -> F + 'static,
    ) -> &F {
        todo!()
    }
}

pub use ecs::*;
mod ecs {
    use super::*;
    pub struct Blah<K, V> {
        _p: PhantomData<(K, V)>,
    }
    pub type EcsModel<K, Ty> = fn(Blah<K, Ty>);
}
