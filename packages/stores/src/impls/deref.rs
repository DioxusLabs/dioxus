use std::ops::DerefMut;

use crate::{store::Store, MappedStore};
use dioxus_signals::Readable;

impl<Lens, T> Store<T, Lens>
where
    Lens: Readable<Target = T> + 'static,
    T: DerefMut + 'static,
{
    /// Returns a store that dereferences the original value. The dereferenced store shares the same
    /// subscriptions and tracking as the original store, but allows you to access the methods of the underlying type.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Box::new(vec![1, 2, 3]));
    /// let deref_store = store.deref();
    /// // The dereferenced store can access the store methods of the underlying type.
    /// assert_eq!(deref_store.len(), 3);
    /// ```
    pub fn deref(self) -> MappedStore<T::Target, Lens> {
        let map: fn(&T) -> &T::Target = |value| value.deref();
        let map_mut: fn(&mut T) -> &mut T::Target = |value| value.deref_mut();
        self.into_selector().map(map, map_mut).into()
    }
}
