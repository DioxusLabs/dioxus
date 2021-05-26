use std::rc::Rc;

use crate::Atom;

pub struct RecoilApi {}

impl RecoilApi {
    /// Get the value of an atom. Returns a reference to the underlying data.
    pub fn get<T: PartialEq>(&self, t: &'static Atom<T>) -> Rc<T> {
        todo!()
    }

    /// Replace an existing value with a new value
    ///
    /// This does not replace the value instantly.
    /// All calls to "get" will return the old value until the component is rendered.
    pub fn set<T: PartialEq>(&self, t: &'static Atom<T>, new: T) {
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
