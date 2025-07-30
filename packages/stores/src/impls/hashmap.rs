use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

use crate::store::Store;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

impl<
        W: Writable<Target = HashMap<K, V, St>> + Copy + 'static,
        K: 'static,
        V: 'static,
        St: 'static,
    > Store<HashMap<K, V, St>, W>
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
    pub fn len(self) -> usize {
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
    pub fn is_empty(self) -> bool {
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
        self,
    ) -> impl Iterator<
        Item = (
            K,
            Store<
                V,
                MappedMutSignal<
                    V,
                    W,
                    impl Fn(&HashMap<K, V, St>) -> &V + Copy + 'static,
                    impl Fn(&mut HashMap<K, V, St>) -> &mut V + Copy + 'static,
                >,
            >,
        ),
    >
    where
        K: Copy + Eq + Hash,
        St: BuildHasher,
    {
        self.selector().track_shallow();
        let keys = self.selector().peek().keys().cloned().collect::<Vec<_>>();
        keys.into_iter()
            .map(move |key| (key, self.get(key).unwrap()))
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
        self,
    ) -> impl Iterator<
        Item = Store<
            V,
            MappedMutSignal<
                V,
                W,
                impl Fn(&HashMap<K, V, St>) -> &V + Copy + 'static,
                impl Fn(&mut HashMap<K, V, St>) -> &mut V + Copy + 'static,
            >,
        >,
    >
    where
        K: Copy + Eq + Hash,
        St: BuildHasher,
    {
        self.selector().track_shallow();
        let keys = self.selector().peek().keys().cloned().collect::<Vec<_>>();
        keys.into_iter()
            .map(|k| k.borrow().clone())
            .map(move |key| self.get(key).unwrap())
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
    {
        self.selector().mark_dirty_shallow();
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
    pub fn clear(&mut self) {
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
    pub fn retain(&mut self, mut f: impl FnMut(&K, &V) -> bool) {
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
    pub fn get<Q>(
        self,
        key: Q,
    ) -> Option<
        Store<
            V,
            MappedMutSignal<
                V,
                W,
                impl Fn(&HashMap<K, V, St>) -> &V + Copy + 'static,
                impl Fn(&mut HashMap<K, V, St>) -> &mut V + Copy + 'static,
            >,
        >,
    >
    where
        Q: Hash + Eq + Copy + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.contains_key(&key).then(|| {
            let key_ = key.clone();
            self.selector()
                .hash_child(
                    key.borrow(),
                    move |value| value.get(&key).unwrap(),
                    move |value| value.get_mut(&key_).unwrap(),
                )
                .into()
        })
    }
}
