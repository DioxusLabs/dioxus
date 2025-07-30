use std::{hash::Hash, ops::IndexMut};

use crate::store::Store;
use dioxus_signals::{MappedMutSignal, Writable};

impl<W, T> Store<T, W> {
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
    pub fn index<Idx>(
        self,
        index: Idx,
    ) -> Store<
        T::Output,
        MappedMutSignal<
            T::Output,
            W,
            impl Fn(&T) -> &T::Output + Copy + 'static,
            impl Fn(&mut T) -> &mut T::Output + Copy + 'static,
        >,
    >
    where
        T: IndexMut<Idx> + 'static,
        Idx: Hash + Copy + 'static,
        W: Writable<Target = T> + Copy + 'static,
    {
        self.selector()
            .hash_child(
                index,
                move |value| value.index(index),
                move |value| value.index_mut(index),
            )
            .into()
    }
}
