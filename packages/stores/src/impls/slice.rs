//! `Store<Vec<T>, _>` read-side methods live on the
//! [`ProjectSlice`](crate::ProjectSlice) trait. The `iter` / `get` methods
//! stay here because they produce a Store-specific
//! [`IndexWrite`](crate::impls::index::IndexWrite)-backed type.

use std::iter::FusedIterator;

use crate::{impls::index::IndexWrite, store::Store, ProjectSlice};
use dioxus_signals::Readable;

impl<Lens, I> Store<Vec<I>, Lens>
where
    Lens: Readable<Target = Vec<I>> + Copy + 'static,
    I: 'static,
{
    /// Iterate items, producing one indexed store per element.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = Store<I, IndexWrite<usize, Lens>>>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        Lens: Clone,
    {
        let len = ProjectSlice::len(self);
        (0..len).map(move |i| (*self).index(i))
    }

    /// Try to get the item at `index` as a store.
    pub fn get(&self, index: usize) -> Option<Store<I, IndexWrite<usize, Lens>>>
    where
        Lens: Clone,
    {
        if index >= ProjectSlice::len(self) {
            None
        } else {
            Some((*self).index(index))
        }
    }
}
