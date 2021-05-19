// ========================
//   Important hooks
// ========================

pub fn init_recoil_root(ctx: Context) {}

pub fn use_recoil_value() {}

pub fn use_recoil() {}

pub fn use_set_recoil() {}

use dioxus_core::virtual_dom::Context;

pub struct RecoilContext {}

impl RecoilContext {
    /// Get the value of an atom. Returns a reference to the underlying data.
    pub fn get<T: PartialEq>(&self, t: &'static Atom<T>) -> &T {
        todo!()
    }

    /// Replace an existing value with a new value
    ///
    /// This does not replace the value instantly.
    /// All calls to "get" will return the old value until the component is rendered.
    pub fn set<T: PartialEq, O>(&self, t: &'static Atom<T>, new: T) {
        self.modify(t, move |old| *old = new);
    }

    /// Modify lets you modify the value in place. However, because there's no previous value around to compare
    /// the new one with, we are unable to memoize the change. As such, all downsteam users of this Atom will
    /// be updated, causing all subsrcibed components to re-render.
    ///
    /// This is fine for most values, but might not be performant when dealing with collections. For collections,
    /// use the "Family" variants as these will stay memoized for inserts, removals, and modifications.
    ///
    /// Note - like "set" this won't propogate instantly. Once all "gets" are dropped, only then will the update occur
    pub fn modify<T: PartialEq, O>(&self, t: &'static Atom<T>, f: impl FnOnce(&mut T) -> O) -> O {
        todo!()
    }
}

pub fn use_recoil_context<T>(c: Context) -> &T {
    todo!()
}

// pub fn use_callback<'a>(c: &Context<'a>, f: impl Fn() -> G) -> &'a RecoilContext {
//     todo!()
// }

pub fn use_atom<'a, T: PartialEq>(c: Context<'a>, t: &'static Atom<T>) -> &'a T {
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

pub struct AtomBuilder<T: PartialEq> {
    pub key: String,
    pub manual_init: Option<Box<dyn Fn() -> T>>,
    _never: std::marker::PhantomData<T>,
}

impl<T: PartialEq> AtomBuilder<T> {
    pub fn new() -> Self {
        Self {
            key: "".to_string(),
            manual_init: None,
            _never: std::marker::PhantomData {},
        }
    }

    pub fn init<A: Fn() -> T + 'static>(&mut self, f: A) {
        self.manual_init = Some(Box::new(f));
    }

    pub fn set_key(&mut self, _key: &'static str) {}
}

// =====================================
//    Atom
// =====================================
pub struct atom<T: PartialEq>(pub fn(&mut AtomBuilder<T>) -> T);
pub type Atom<T: PartialEq> = atom<T>;

// =====================================
//    Atom Family
// =====================================
pub struct AtomFamilyBuilder<K, V> {
    _never: std::marker::PhantomData<(K, V)>,
}

pub struct atom_family<K: PartialEq, V: PartialEq>(pub fn(&mut AtomFamilyBuilder<K, V>));
pub type AtomFamily<K: PartialEq, V: PartialEq> = atom_family<K, V>;

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

pub fn use_selector<'a, O>(c: Context<'a>, s: &'static Selector<O>) -> &'a O {
    todo!()
}
