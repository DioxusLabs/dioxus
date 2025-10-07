//! Additional utilities for indexing into stores.

use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    ops::{self, Index, IndexMut},
};

use crate::{scope::SelectorScope, store::Store, ReadStore};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, ReadSignal, Readable, UnsyncStorage, Writable,
    WriteLock, WriteSignal,
};

/// The way a data structure index into its children based on a key. The selector must use this indexing
/// method consistently to ensure that the same key always maps to the same child.
pub trait IndexSelector<Idx> {
    /// Given a selector and an index, scope the selector to the child at the given index.
    fn scope_selector<Lens>(selector: SelectorScope<Lens>, index: &Idx) -> SelectorScope<Lens>;
}

impl<T> IndexSelector<usize> for Vec<T> {
    fn scope_selector<Lens>(selector: SelectorScope<Lens>, index: &usize) -> SelectorScope<Lens> {
        selector.child_unmapped(*index as _)
    }
}

impl<T> IndexSelector<usize> for [T] {
    fn scope_selector<Lens>(selector: SelectorScope<Lens>, index: &usize) -> SelectorScope<Lens> {
        selector.child_unmapped(*index as _)
    }
}

impl<K, V, I> IndexSelector<I> for HashMap<K, V>
where
    I: Hash,
{
    fn scope_selector<Lens>(selector: SelectorScope<Lens>, index: &I) -> SelectorScope<Lens> {
        selector.hash_child_unmapped(&index)
    }
}

impl<K, V, I> IndexSelector<I> for BTreeMap<K, V>
where
    I: Hash,
{
    fn scope_selector<Lens>(selector: SelectorScope<Lens>, index: &I) -> SelectorScope<Lens> {
        selector.hash_child_unmapped(&index)
    }
}

impl<Lens, T> Store<T, Lens> {
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
    pub fn index<Idx>(self, index: Idx) -> Store<T::Output, IndexWrite<Idx, Lens>>
    where
        T: IndexMut<Idx> + 'static + IndexSelector<Idx>,
        Lens: Readable<Target = T> + 'static,
    {
        T::scope_selector(self.into_selector(), &index)
            .map_writer(move |write| IndexWrite { index, write })
            .into()
    }
}

/// A specific index in a `Readable` / `Writable` type
#[derive(Clone, Copy)]
pub struct IndexWrite<Index, Write> {
    index: Index,
    write: Write,
}

impl<Index, Write> Readable for IndexWrite<Index, Write>
where
    Write: Readable,
    Write::Target: ops::Index<Index> + 'static,
    Index: Clone,
{
    type Target = <Write::Target as ops::Index<Index>>::Output;

    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().map(|value| {
            Self::Storage::map(value, |value: &Write::Target| {
                value.index(self.index.clone())
            })
        })
    }

    fn try_peek_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().map(|value| {
            Self::Storage::map(value, |value: &Write::Target| {
                value.index(self.index.clone())
            })
        })
    }

    fn subscribers(&self) -> dioxus_core::Subscribers
    where
        Self::Target: 'static,
    {
        self.write.subscribers()
    }
}

impl<Index, Write> Writable for IndexWrite<Index, Write>
where
    Write: Writable,
    Write::Target: ops::IndexMut<Index> + 'static,
    Index: Clone,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> Result<dioxus_signals::WritableRef<'static, Self>, BorrowMutError>
    where
        Self::Target: 'static,
    {
        self.write.try_write_unchecked().map(|value| {
            WriteLock::map(value, |value: &mut Write::Target| {
                value.index_mut(self.index.clone())
            })
        })
    }
}

impl<Idx, T, Write> ::std::convert::From<Store<T, IndexWrite<Idx, Write>>>
    for Store<T, WriteSignal<T>>
where
    Write: Writable<Storage = UnsyncStorage> + 'static,
    Write::WriteMetadata: 'static,
    Write::Target: ops::IndexMut<Idx, Output = T> + 'static,
    Idx: Clone + 'static,
    T: 'static,
{
    fn from(value: Store<T, IndexWrite<Idx, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| WriteSignal::new(writer))
            .into()
    }
}

impl<Idx, T, Write> ::std::convert::From<Store<T, IndexWrite<Idx, Write>>> for ReadStore<T>
where
    Write: Readable<Storage = UnsyncStorage> + 'static,
    Write::Target: ops::Index<Idx, Output = T> + 'static,
    Idx: Clone + 'static,
    T: 'static,
{
    fn from(value: Store<T, IndexWrite<Idx, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| ReadSignal::new(writer))
            .into()
    }
}
