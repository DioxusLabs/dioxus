use std::{
    borrow::Borrow,
    collections::HashSet,
    hash::Hash,
    sync::{Mutex, OnceLock},
};

pub(crate) struct InternSet<T> {
    inner: OnceLock<Mutex<HashSet<T>>>,
}

impl<T> InternSet<T>
where
    T: Clone + Eq + Hash,
{
    pub(crate) const fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    pub(crate) fn get_or_insert_with<Q>(&self, key: &Q, create: impl FnOnce() -> T) -> T
    where
        T: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let values = self.inner.get_or_init(|| Mutex::new(HashSet::new()));
        if let Some(value) = values.lock().unwrap().get(key) {
            return value.clone();
        }

        let value = create();
        let mut values = values.lock().unwrap();
        if let Some(value) = values.get(key) {
            return value.clone();
        }
        values.insert(value.clone());
        value
    }
}
