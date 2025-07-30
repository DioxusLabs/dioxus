use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

use crate::store::Store;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

pub trait HashMapStoreExt {
    type Key;
    type Value;
    type State;
    type Write;

    fn len(self) -> usize;

    fn is_empty(self) -> bool;

    fn get<Q>(
        self,
        key: Q,
    ) -> Store<
        Self::Value,
        MappedMutSignal<
            Self::Value,
            Self::Write,
            impl Fn(&HashMap<Self::Key, Self::Value, Self::State>) -> &Self::Value + Copy + 'static,
            impl Fn(&mut HashMap<Self::Key, Self::Value, Self::State>) -> &mut Self::Value
                + Copy
                + 'static,
        >,
    >
    where
        Q: Hash + Eq + Copy + 'static,
        Self::Key: Borrow<Q> + Eq + Hash,
        Self::State: BuildHasher;

    fn iter(
        self,
    ) -> impl Iterator<
        Item = (
            Self::Key,
            Store<
                Self::Value,
                MappedMutSignal<
                    Self::Value,
                    Self::Write,
                    impl Fn(&HashMap<Self::Key, Self::Value, Self::State>) -> &Self::Value
                        + Copy
                        + 'static,
                    impl Fn(&mut HashMap<Self::Key, Self::Value, Self::State>) -> &mut Self::Value
                        + Copy
                        + 'static,
                >,
            >,
        ),
    >
    where
        Self::Key: Copy + Eq + Hash,
        Self::State: BuildHasher;

    fn values(
        self,
    ) -> impl Iterator<
        Item = Store<
            Self::Value,
            MappedMutSignal<
                Self::Value,
                Self::Write,
                impl Fn(&HashMap<Self::Key, Self::Value, Self::State>) -> &Self::Value + Copy + 'static,
                impl Fn(&mut HashMap<Self::Key, Self::Value, Self::State>) -> &mut Self::Value
                    + Copy
                    + 'static,
            >,
        >,
    >
    where
        Self::Key: Copy + Eq + Hash,
        Self::State: BuildHasher;

    fn insert(self, key: Self::Key, value: Self::Value)
    where
        Self::Key: Eq + Hash,
        Self::State: BuildHasher;

    fn remove<Q>(self, key: &Q) -> Option<Self::Value>
    where
        Q: Hash + Eq + Copy + 'static,
        Self::Key: Borrow<Q> + Eq + Hash,
        Self::State: BuildHasher;

    fn clear(self);

    fn retain(self, f: impl FnMut(&Self::Key, &mut Self::Value) -> bool);
}

impl<
        W: Writable<Target = HashMap<K, V, St>> + Copy + 'static,
        K: 'static,
        V: 'static,
        St: 'static,
    > HashMapStoreExt for Store<HashMap<K, V, St>, W>
{
    type Key = K;
    type Value = V;
    type State = St;
    type Write = W;

    fn len(self) -> usize {
        self.selector().track();
        self.selector().write.read().len()
    }

    fn is_empty(self) -> bool {
        self.selector().track();
        self.selector().write.read().is_empty()
    }

    fn iter(
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
        self.selector().track();
        let keys = self
            .selector()
            .write
            .read()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter().map(move |key| (key, self.get(key)))
    }

    fn values(
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
        self.selector().track();
        let keys = self
            .selector()
            .write
            .read()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter()
            .map(|k| *k.borrow())
            .map(move |key| self.get(key))
    }

    fn insert(self, key: K, value: V)
    where
        K: Eq + Hash,
        St: BuildHasher,
    {
        self.selector().mark_dirty_shallow();
        self.selector().write.write_unchecked().insert(key, value);
    }

    fn remove<Q>(self, key: &Q) -> Option<V>
    where
        Q: Hash + Eq + Copy + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.selector().mark_dirty_shallow();
        self.selector().write.write_unchecked().remove(key)
    }

    fn clear(self) {
        self.selector().mark_dirty_shallow();
        self.selector().write.write_unchecked().clear();
    }

    fn retain(self, f: impl FnMut(&K, &mut V) -> bool) {
        self.selector().mark_dirty_shallow();
        self.selector().write.write_unchecked().retain(f);
    }

    fn get<Q>(
        self,
        key: Q,
    ) -> Store<
        V,
        MappedMutSignal<
            V,
            W,
            impl Fn(&HashMap<K, V, St>) -> &V + Copy + 'static,
            impl Fn(&mut HashMap<K, V, St>) -> &mut V + Copy + 'static,
        >,
    >
    where
        Q: Hash + Eq + Copy + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.selector()
            .hash_scope(
                key.borrow(),
                move |value| value.get(&key).unwrap(),
                move |value| value.get_mut(&key).unwrap(),
            )
            .into()
    }
}
