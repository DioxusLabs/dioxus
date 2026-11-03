use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{Mutex, OnceLock},
};

/// A lazily-initialized, thread-safe key-to-value cache used to intern
/// compiled (and leaked) template data.
pub(crate) struct InternMap<K, V> {
    inner: OnceLock<Mutex<HashMap<K, V>>>,
}

impl<K, V> InternMap<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub(crate) const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    /// Look up `key`, creating and caching the value when missing. `create`
    /// runs without the lock held so it may recursively intern through the
    /// same map.
    pub(crate) fn get_or_insert_with<Q>(&self, key: &Q, create: impl FnOnce() -> V) -> V
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ToOwned<Owned = K> + ?Sized,
    {
        let values = self.inner.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(value) = values.lock().unwrap().get(key) {
            return value.clone();
        }

        let value = create();
        let mut values = values.lock().unwrap();
        if let Some(existing) = values.get(key) {
            return existing.clone();
        }
        values.insert(key.to_owned(), value.clone());
        value
    }
}
