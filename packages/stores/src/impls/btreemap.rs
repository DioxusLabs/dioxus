//! `Store<BTreeMap<K, V>, _>` — shape-agnostic methods live on the
//! [`ProjectBTreeMap`](crate::ProjectBTreeMap) /
//! [`ProjectBTreeMapMut`](crate::ProjectBTreeMapMut) traits. The store-specific
//! `iter` / `values` / `get` / `get_unchecked` (producing `GetWrite`-backed
//! stores) and the `GetWrite` lens stay here.

use std::{
    borrow::Borrow, collections::BTreeMap, hash::Hash, iter::FusedIterator, panic::Location,
};

use crate::{store::Store, ProjectBTreeMap, ReadStore};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, ReadSignal, Readable, ReadableExt, UnsyncStorage,
    Writable, WriteLock, WriteSignal,
};
use generational_box::ValueDroppedError;

impl<Lens, K, V> Store<BTreeMap<K, V>, Lens>
where
    Lens: Readable<Target = BTreeMap<K, V>> + Copy + 'static,
    K: 'static,
    V: 'static,
{
    /// Iterate the map, producing one store per value.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (K, Store<V, GetWrite<K, Lens>>)>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Hash + Ord + Clone,
        Lens: Clone,
    {
        ProjectBTreeMap::<K, V>::len(self);
        let keys: Vec<_> = self.selector().peek_unchecked().keys().cloned().collect();
        keys.into_iter().map(move |key| {
            let value = (*self).get_unchecked(key.clone());
            (key, value)
        })
    }

    /// Iterate the map values as stores.
    pub fn values(
        &self,
    ) -> impl ExactSizeIterator<Item = Store<V, GetWrite<K, Lens>>>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Hash + Ord + Clone,
        Lens: Clone,
    {
        ProjectBTreeMap::<K, V>::len(self);
        let keys = self.selector().peek().keys().cloned().collect::<Vec<_>>();
        keys.into_iter().map(move |key| (*self).get_unchecked(key))
    }

    /// Get a store for the value at `key` if it exists.
    pub fn get<Q>(self, key: Q) -> Option<Store<V, GetWrite<Q, Lens>>>
    where
        Q: Hash + Ord + 'static,
        K: Borrow<Q> + Ord,
    {
        ProjectBTreeMap::contains_key(&self, &key).then(|| self.get_unchecked(key))
    }

    /// Get a store for the value at `key` without checking existence.
    #[track_caller]
    pub fn get_unchecked<Q>(self, key: Q) -> Store<V, GetWrite<Q, Lens>>
    where
        Q: Hash + Ord + 'static,
        K: Borrow<Q> + Ord,
    {
        let created = Location::caller();
        self.into_selector()
            .hash_child_unmapped(key.borrow())
            .map_writer(move |writer| GetWrite {
                index: key,
                write: writer,
                created,
            })
            .into()
    }
}

/// A specific index in a `Readable` / `Writable` BTreeMap.
#[derive(Clone, Copy)]
pub struct GetWrite<Index, Write> {
    index: Index,
    write: Write,
    created: &'static Location<'static>,
}

impl<Index, Write, K, V> Readable for GetWrite<Index, Write>
where
    Write: Readable<Target = BTreeMap<K, V>>,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
{
    type Target = V;
    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn try_peek_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn subscribers(&self) -> dioxus_core::Subscribers
    where
        Self::Target: 'static,
    {
        self.write.subscribers()
    }
}

impl<Index, Write, K, V> Writable for GetWrite<Index, Write>
where
    Write: Writable<Target = BTreeMap<K, V>>,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> Result<dioxus_signals::WritableRef<'static, Self>, BorrowMutError>
    where
        Self::Target: 'static,
    {
        self.write.try_write_unchecked().and_then(|value| {
            WriteLock::filter_map(value, |value: &mut Write::Target| {
                value.get_mut(&self.index)
            })
            .ok_or_else(|| BorrowMutError::Dropped(ValueDroppedError::new(self.created)))
        })
    }
}

impl<Index, Write, K, V> ::std::convert::From<Store<V, GetWrite<Index, Write>>>
    for Store<V, WriteSignal<V>>
where
    Write::WriteMetadata: 'static,
    Write: Writable<Target = BTreeMap<K, V>, Storage = UnsyncStorage> + 'static,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
    V: 'static,
{
    fn from(value: Store<V, GetWrite<Index, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| WriteSignal::new(writer))
            .into()
    }
}

impl<Index, Write, K, V> ::std::convert::From<Store<V, GetWrite<Index, Write>>> for ReadStore<V>
where
    Write: Readable<Target = BTreeMap<K, V>, Storage = UnsyncStorage> + 'static,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
    V: 'static,
{
    fn from(value: Store<V, GetWrite<Index, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| ReadSignal::new(writer))
            .into()
    }
}
