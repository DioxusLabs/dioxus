use crate::{CreateSelector, SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};
use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

impl<K, V, St> Storable for HashMap<K, V, St> {
    type Store<View> = HashMapSelector<View, K, V, St>;
}

pub struct HashMapSelector<W, K, V, St> {
    selector: SelectorScope<W>,
    _phantom: std::marker::PhantomData<(K, V, St)>,
}

impl<W, K, V, St> PartialEq for HashMapSelector<W, K, V, St>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, K, V, St> Clone for HashMapSelector<W, K, V, St>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<W, K, V, St> Copy for HashMapSelector<W, K, V, St> where W: Copy {}

impl<W, K, V, St> CreateSelector for HashMapSelector<W, K, V, St> {
    type View = W;

    fn new(selector: SelectorScope<Self::View>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<
        W: Writable<Target = HashMap<K, V, St>> + Copy + 'static,
        K: 'static,
        V: 'static,
        St: 'static,
    > HashMapSelector<W, K, V, St>
{
    pub fn get<Q>(
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
        V: Storable,
    {
        V::Store::new(self.selector.hash_scope(
            key.borrow(),
            move |value| value.get(&key).unwrap(),
            move |value| value.get_mut(&key).unwrap(),
        ))
    }

    pub fn len(self) -> usize {
        self.selector.track();
        self.selector.write.read().len()
    }

    pub fn is_empty(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_empty()
    }

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
        V: Storable,
    {
        self.selector.track();
        let keys = self
            .selector
            .write
            .read()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter().map(move |key| (key, self.get(key)))
    }

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
        V: Storable,
    {
        self.selector.track();
        let keys = self
            .selector
            .write
            .read()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter()
            .map(|k| *k.borrow())
            .map(move |key| self.get(key))
    }

    pub fn insert(self, key: K, value: V)
    where
        K: Eq + Hash,
        St: BuildHasher,
    {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().insert(key, value);
    }

    pub fn remove<Q>(self, key: &Q) -> Option<V>
    where
        Q: Hash + Eq + Copy + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().remove(key)
    }

    pub fn clear(self) {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().clear();
    }

    pub fn retain(self, f: impl FnMut(&K, &mut V) -> bool) {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().retain(f);
    }
}
