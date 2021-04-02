use dioxus_core::context::Context;

pub struct RecoilContext<T: 'static> {
    _inner: T,
}

impl<T: 'static> RecoilContext<T> {
    /// Get the value of an atom. Returns a reference to the underlying data.

    pub fn get(&self) {}

    /// Replace an existing value with a new value
    ///
    /// This does not replace the value instantly, and all calls to "get" within the current scope will return
    pub fn set(&self) {}

    // Modify lets you modify the value in place. However, because there's no previous value around to compare
    // the new one with, we are unable to memoize the change. As such, all downsteam users of this Atom will
    // be updated, causing all subsrcibed components to re-render.
    //
    // This is fine for most values, but might not be performant when dealing with collections. For collections,
    // use the "Family" variants as these will stay memoized for inserts, removals, and modifications.
    //
    // Note - like "set" this won't propogate instantly. Once all "gets" are dropped, only then will we run the
    pub fn modify(&self) {}
}

pub fn use_callback<'a, G>(c: &Context<'a>, f: impl Fn() -> G) -> &'a RecoilContext<G> {
    todo!()
}

pub fn use_atom<T: PartialEq, O>(c: &Context, t: &'static Atom<T>) -> O {
    todo!()
}
pub fn use_batom<T: PartialEq, O>(c: &Context, t: impl Readable) -> O {
    todo!()
}

pub trait Readable {}
impl<T: PartialEq> Readable for &'static Atom<T> {}
impl<K: PartialEq, V: PartialEq> Readable for &'static AtomFamily<K, V> {}

pub fn use_atom_family<'a, K: PartialEq, V: PartialEq>(
    c: &Context<'a>,
    t: &'static AtomFamily<K, V>,
    g: K,
) -> &'a V {
    todo!()
}

pub use atoms::{atom, Atom};
pub use atoms::{atom_family, AtomFamily};
mod atoms {

    use super::*;
    pub struct AtomBuilder<T: PartialEq> {
        pub key: String,
        pub manual_init: Option<Box<dyn Fn() -> T>>,
        _never: std::marker::PhantomData<T>,
    }

    impl<T: PartialEq> AtomBuilder<T> {
        pub fn new() -> Self {
            Self {
                key: uuid::Uuid::new_v4().to_string(),
                manual_init: None,
                _never: std::marker::PhantomData {},
            }
        }

        pub fn init<A: Fn() -> T + 'static>(&mut self, f: A) {
            self.manual_init = Some(Box::new(f));
        }

        pub fn set_key(&mut self, _key: &'static str) {}
    }

    pub struct atom<T: PartialEq>(pub fn(&mut AtomBuilder<T>) -> T);
    pub type Atom<T: PartialEq> = atom<T>;

    pub struct AtomFamilyBuilder<K, V> {
        _never: std::marker::PhantomData<(K, V)>,
    }

    pub struct atom_family<K: PartialEq, V: PartialEq>(pub fn(&mut AtomFamilyBuilder<K, V>));
    pub type AtomFamily<K: PartialEq, V: PartialEq> = atom_family<K, V>;
}

pub use selectors::selector;
mod selectors {
    pub struct SelectorBuilder<Out, const Built: bool> {
        _p: std::marker::PhantomData<Out>,
    }
    impl<O> SelectorBuilder<O, false> {
        pub fn getter(self, f: impl Fn(()) -> O) -> SelectorBuilder<O, true> {
            std::rc::Rc::pin(value)
            todo!()
        }
    }
    pub struct selector<O>(pub fn(SelectorBuilder<O, false>) -> SelectorBuilder<O, true>);
}
