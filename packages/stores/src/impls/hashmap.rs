//! Additional utilities for `HashMap` stores.

use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
    iter::FusedIterator,
    panic::Location,
};

use crate::{store::Store, ReadStore};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, ReadSignal, Readable, ReadableExt, UnsyncStorage,
    Writable, WriteLock, WriteSignal,
};
use generational_box::ValueDroppedError;

impl<Lens: Readable<Target = HashMap<K, V, St>> + 'static, K: 'static, V: 'static, St: 'static>
    Store<HashMap<K, V, St>, Lens>
{
    /// Get the length of the HashMap. This method will track the store shallowly and only cause
    /// re-runs when items are added or removed from the map, not when existing values are modified.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// assert_eq!(store.len(), 0);
    /// store.insert(0, "value".to_string());
    /// assert_eq!(store.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.selector().track_shallow();
        self.selector().peek().len()
    }

    /// Check if the HashMap is empty. This method will track the store shallowly and only cause
    /// re-runs when items are added or removed from the map, not when existing values are modified.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// assert!(store.is_empty());
    /// store.insert(0, "value".to_string());
    /// assert!(!store.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.selector().track_shallow();
        self.selector().peek().is_empty()
    }

    /// Iterate over the current entries in the HashMap, returning a tuple of the key and a store for the value. This method
    /// will track the store shallowly and only cause re-runs when items are added or removed from the map, not when existing
    /// values are modified.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// store.insert(0, "value1".to_string());
    /// store.insert(1, "value2".to_string());
    /// for (key, value_store) in store.iter() {
    ///     println!("{}: {}", key, value_store.read());
    /// }
    /// ```
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
        self.selector().track_shallow();
        let keys: Vec<_> = self.selector().peek_unchecked().keys().cloned().collect();
        keys.into_iter()
            .map(move |key| (key.clone(), self.clone().get_unchecked(key)))
    }

    /// Get an iterator over the values in the HashMap. This method will track the store shallowly and only cause
    /// re-runs when items are added or removed from the map, not when existing values are modified.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// store.insert(0, "value1".to_string());
    /// store.insert(1, "value2".to_string());
    /// for value_store in store.values() {
    ///     println!("{}", value_store.read());
    /// }
    /// ```
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
        self.selector().track_shallow();
        let keys = self.selector().peek().keys().cloned().collect::<Vec<_>>();
        keys.into_iter()
            .map(move |key| self.clone().get_unchecked(key))
    }

    /// Insert a new key-value pair into the HashMap. This method will mark the store as shallowly dirty, causing
    /// re-runs of any reactive scopes that depend on the shape of the map.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// assert!(store.get(0).is_none());
    /// store.insert(0, "value".to_string());
    /// assert_eq!(store.get(0).unwrap().cloned(), "value".to_string());
    /// ```
    pub fn insert(&mut self, key: K, value: V)
    where
        K: Eq + Hash,
        St: BuildHasher,
        Lens: Writable,
    {
        // Mark the store itself as dirty since the keys may have changed
        self.selector().mark_dirty_shallow();
        // Mark the existing value as dirty if it exists
        self.selector()
            .as_ref()
            .hash_child_unmapped(key.borrow())
            .mark_dirty();
        self.selector().write_untracked().insert(key, value);
    }

    /// Remove a key-value pair from the HashMap. This method will mark the store as shallowly dirty, causing
    /// re-runs of any reactive scopes that depend on the shape of the map or the value of the removed key.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// store.insert(0, "value".to_string());
    /// assert_eq!(store.get(0).unwrap().cloned(), "value".to_string());
    /// let removed_value = store.remove(&0);
    /// assert_eq!(removed_value, Some("value".to_string()));
    /// assert!(store.get(0).is_none());
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
        Lens: Writable,
    {
        self.selector().mark_dirty_shallow();
        self.selector().write_untracked().remove(key)
    }

    /// Clear the HashMap, removing all key-value pairs. This method will mark the store as shallowly dirty,
    /// causing re-runs of any reactive scopes that depend on the shape of the map.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// store.insert(1, "value1".to_string());
    /// store.insert(2, "value2".to_string());
    /// assert_eq!(store.len(), 2);
    /// store.clear();
    /// assert!(store.is_empty());
    /// ```
    pub fn clear(&mut self)
    where
        Lens: Writable,
    {
        self.selector().mark_dirty_shallow();
        self.selector().write_untracked().clear();
    }

    /// Retain only the key-value pairs that satisfy the given predicate. This method will mark the store as shallowly dirty,
    /// causing re-runs of any reactive scopes that depend on the shape of the map or the values retained.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// store.insert(1, "value1".to_string());
    /// store.insert(2, "value2".to_string());
    /// store.retain(|key, value| *key == 1);
    /// assert_eq!(store.len(), 1);
    /// assert!(store.get(1).is_some());
    /// assert!(store.get(2).is_none());
    /// ```
    pub fn retain(&mut self, mut f: impl FnMut(&K, &V) -> bool)
    where
        Lens: Writable,
    {
        self.selector().mark_dirty_shallow();
        self.selector().write_untracked().retain(|k, v| f(k, v));
    }

    /// Check if the HashMap contains a key. This method will track the store shallowly and only cause
    /// re-runs when items are added or removed from the map, not when existing values are modified.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// assert!(!store.contains_key(&0));
    /// store.insert(0, "value".to_string());
    /// assert!(store.contains_key(&0));
    /// ```
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.selector().track_shallow();
        self.selector().peek().contains_key(key)
    }

    /// Get a store for the value associated with the given key. This method creates a new store scope
    /// that tracks just changes to the value associated with the key.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// assert!(store.get(0).is_none());
    /// store.insert(0, "value".to_string());
    /// assert_eq!(store.get(0).unwrap().cloned(), "value".to_string());
    /// ```
    pub fn get<Q>(self, key: Q) -> Option<Store<V, GetWrite<Q, Lens>>>
    where
        Q: Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.contains_key(&key).then(|| self.get_unchecked(key))
    }

    /// Get a store for the value associated with the given key without checking if the key exists.
    /// This method creates a new store scope that tracks just changes to the value associated with the key.
    ///
    /// This is not unsafe, but it will panic when you try to read the value if it does not exist.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// use dioxus::prelude::*;
    /// use std::collections::HashMap;
    /// let mut store = use_store(|| HashMap::new());
    /// store.insert(0, "value".to_string());
    /// assert_eq!(store.get_unchecked(0).cloned(), "value".to_string());
    /// ```
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

/// A specific index in a `Readable` / `Writable` hashmap
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
