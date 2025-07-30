use std::{hash::Hash, ops::IndexMut};

use crate::store::Store;
use dioxus_signals::{MappedMutSignal, Writable};

mod private {
    pub trait Sealed {}
}

/// A extension trait for `Store` for types that implement `IndexMut`.
///
/// # Example
/// ```rust, no_run
/// use dioxus_stores::*;
/// let store = use_store(|| vec![1, 2, 3]);
/// let indexed_store = store.index(1);
/// // The indexed store can access the store methods of the indexed store.
/// assert_eq!(indexed_store(), 2);
/// ```
pub trait IndexStoreExt<Idx>: private::Sealed {
    /// The collection type of the store.
    type Collection;
    /// The writer backing the store
    type Write;
    /// The item type of the store.
    type Item;

    /// Index into the store, returning a store that allows access to the item at the given index. The
    /// new store will only update when the item at the index changes.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// let indexed_store = store.index(1);
    /// // The indexed store can access the store methods of the indexed store.
    /// assert_eq!(indexed_store(), 2);
    /// ```
    fn index(
        self,
        index: Idx,
    ) -> Store<
        Self::Item,
        MappedMutSignal<
            Self::Item,
            Self::Write,
            impl Fn(&Self::Collection) -> &Self::Item + Copy + 'static,
            impl Fn(&mut Self::Collection) -> &mut Self::Item + Copy + 'static,
        >,
    >;
}

impl<W, T> private::Sealed for Store<T, W> {}

impl<W, T, I, Idx> IndexStoreExt<Idx> for Store<T, W>
where
    W: Writable<Target = T> + Copy + 'static,
    T: IndexMut<Idx, Output = I> + 'static,
    I: 'static,
    Idx: Hash + Copy + 'static,
{
    type Collection = T;
    type Write = W;
    type Item = I;

    fn index(
        self,
        index: Idx,
    ) -> Store<
        Self::Item,
        MappedMutSignal<
            Self::Item,
            W,
            impl Fn(&T) -> &Self::Item + Copy + 'static,
            impl Fn(&mut T) -> &mut Self::Item + Copy + 'static,
        >,
    > {
        self.selector()
            .hash_child(
                index,
                move |value| value.index(index),
                move |value| value.index_mut(index),
            )
            .into()
    }
}
