use std::ops::{Deref, DerefMut};

use crate::store::Store;
use dioxus_signals::{MappedMutSignal, Writable};

mod private {
    pub trait Sealed {}
}

/// A extension trait for `Store` for types that implement `Deref` and `DerefMut`.
///
/// ```rust, no_run
/// use dioxus_stores::*;
/// let store = use_store(|| Box::new(vec![1, 2, 3]));
/// let deref_store = store.deref();
/// // The dereferenced store can access the store methods of the underlying type.
/// assert_eq!(deref_store.len(), 3);
/// ```
pub trait DerefStoreExt: private::Sealed {
    /// The writer backing the store
    type Write;
    /// The original derefable value
    type Value;
    /// The target type of the deref
    type Target: ?Sized;

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
    fn deref(
        self,
    ) -> Store<
        Self::Target,
        MappedMutSignal<
            Self::Target,
            Self::Write,
            fn(&Self::Value) -> &Self::Target,
            fn(&mut Self::Value) -> &mut Self::Target,
        >,
    >;
}

impl<W, T> private::Sealed for Store<T, W>
where
    W: Writable<Target = T> + Copy + 'static,
    T: DerefMut + 'static,
{
}

impl<W, T> DerefStoreExt for Store<T, W>
where
    W: Writable<Target = T> + Copy + 'static,
    T: DerefMut + 'static,
{
    type Write = W;
    type Target = <T as Deref>::Target;
    type Value = T;

    fn deref(self) -> Store<Self::Target, MappedMutSignal<Self::Target, Self::Write>> {
        let map: fn(&T) -> &Self::Target = |value| value.deref();
        let map_mut: fn(&mut T) -> &mut Self::Target = |value| value.deref_mut();
        self.selector().map(map, map_mut).into()
    }
}
