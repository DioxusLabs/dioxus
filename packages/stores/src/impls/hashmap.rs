//! `Store<HashMap<K, V, St>, _>` — shape-agnostic methods live on the
//! [`ProjectHashMap`](crate::ProjectHashMap) and
//! [`ProjectHashMapMut`](crate::ProjectHashMapMut) traits. The store-specific
//! `iter` / `values` / `get` / `get_unchecked` (producing `GetWrite`-backed
//! stores) and the `GetWrite` lens itself stay here.

use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
    iter::FusedIterator,
    panic::Location,
};

use crate::{store::Store, ProjectHashMap, ReadStore};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, ReadSignal, Readable, ReadableExt, UnsyncStorage,
    Writable, WriteLock, WriteSignal,
};
use generational_box::ValueDroppedError;

impl<Lens, K, V, St> Store<HashMap<K, V, St>, Lens>
where
    Lens: Readable<Target = HashMap<K, V, St>> + Copy + 'static,
    K: 'static,
    V: 'static,
    St: 'static,
{
    /// Iterate entries as `(key, value-store)` pairs.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (K, Store<V, GetWrite<K, Lens>>)>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Eq + Hash + Clone,
        St: BuildHasher,
        Lens: Clone,
    {
        ProjectHashMap::<K, V, St>::len(self);
        let keys: Vec<_> = self.selector().peek_unchecked().keys().cloned().collect();
        keys.into_iter()
            .map(move |key| (key.clone(), (*self).get_unchecked(key)))
    }

    /// Iterate values as stores.
    pub fn values(
        &self,
    ) -> impl ExactSizeIterator<Item = Store<V, GetWrite<K, Lens>>>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Eq + Hash + Clone,
        St: BuildHasher,
        Lens: Clone,
    {
        ProjectHashMap::<K, V, St>::len(self);
        let keys = self.selector().peek().keys().cloned().collect::<Vec<_>>();
        keys.into_iter().map(move |key| (*self).get_unchecked(key))
    }

    /// Get a store for the value associated with `key`.
    pub fn get<Q>(self, key: Q) -> Option<Store<V, GetWrite<Q, Lens>>>
    where
        Q: Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        ProjectHashMap::contains_key(&self, &key).then(|| self.get_unchecked(key))
    }

    /// Get a store for the value at `key` without existence check.
    #[track_caller]
    pub fn get_unchecked<Q>(self, key: Q) -> Store<V, GetWrite<Q, Lens>>
    where
        Q: Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        let location = Location::caller();
        self.into_selector()
            .hash_child_unmapped(key.borrow())
            .map_writer(move |writer| GetWrite {
                index: key,
                write: writer,
                created: location,
            })
            .into()
    }
}

/// A specific index in a `Readable` / `Writable` hashmap.
#[derive(Clone, Copy)]
pub struct GetWrite<Index, Write> {
    index: Index,
    write: Write,
    created: &'static Location<'static>,
}

impl<Index, Write, K, V, St> Readable for GetWrite<Index, Write>
where
    Write: Readable<Target = HashMap<K, V, St>>,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
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

impl<Index, Write, K, V, St> Writable for GetWrite<Index, Write>
where
    Write: Writable<Target = HashMap<K, V, St>>,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
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

impl<Index, Write, K, V, St> ::std::convert::From<Store<V, GetWrite<Index, Write>>>
    for Store<V, WriteSignal<V>>
where
    Write::WriteMetadata: 'static,
    Write: Writable<Target = HashMap<K, V, St>, Storage = UnsyncStorage> + 'static,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
    V: 'static,
{
    fn from(value: Store<V, GetWrite<Index, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| WriteSignal::new(writer))
            .into()
    }
}

impl<Index, Write, K, V, St> ::std::convert::From<Store<V, GetWrite<Index, Write>>> for ReadStore<V>
where
    Write: Readable<Target = HashMap<K, V, St>, Storage = UnsyncStorage> + 'static,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
    V: 'static,
{
    fn from(value: Store<V, GetWrite<Index, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| ReadSignal::new(writer))
            .into()
    }
}
