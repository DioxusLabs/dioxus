use std::{
    borrow::Borrow,
    cell::Ref,
    collections::{BTreeMap, HashMap},
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

use dioxus_signals::{UnsyncStorage, WriteLock};
use generational_box::GenerationalRef;

use crate::{
    combinator::{
        Combinator, ReadProjection, ReadProjectionOpt, Transform, ValueProjection, WriteProjection,
        WriteProjectionOpt,
    },
    signal::{Optic, Optional},
};

/// Flatten `Option<Option<X>>` into `Option<X>`.
#[derive(Clone, Copy, Default)]
pub struct FlattenSomeOp;

/// Carrier alias for a single `Option` flattening step.
pub type FlattenSome<A> = Combinator<A, FlattenSomeOp>;

impl<X> Transform<FlattenSomeOp> for Option<X> {
    type Input = Option<Option<X>>;

    fn transform(input: Self::Input, _: &FlattenSomeOp) -> Self {
        input.flatten()
    }
}

/// Iterable view over a projected `Vec<T>`.
pub struct EachVec<A, T> {
    pub(crate) parent: A,
    pub(crate) _marker: PhantomData<fn() -> T>,
}

impl<A: Clone, T> Clone for EachVec<A, T> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            _marker: PhantomData,
        }
    }
}

/// Indexed child carrier inside [`EachVec`].
pub struct VecIndex<A, T> {
    parent: A,
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<A: Clone, T> Clone for VecIndex<A, T> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            index: self.index,
            _marker: PhantomData,
        }
    }
}

/// Optional indexed child carrier inside [`EachVec`].
pub struct VecGet<A, T> {
    parent: A,
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<A: Clone, T> Clone for VecGet<A, T> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            index: self.index,
            _marker: PhantomData,
        }
    }
}

impl<A, T> ReadProjection<T> for VecIndex<A, T>
where
    A: ReadProjection<Vec<T>>,
    T: 'static,
{
    fn read_projection(&self) -> GenerationalRef<Ref<'static, T>> {
        let index = self.index;
        GenerationalRef::map(self.parent.read_projection(), |r| {
            Ref::map(r, |v| &v[index])
        })
    }
}

impl<A, T> WriteProjection<T> for VecIndex<A, T>
where
    A: WriteProjection<Vec<T>>,
    T: 'static,
{
    fn write_projection(&self) -> WriteLock<'static, T, UnsyncStorage> {
        let index = self.index;
        WriteLock::map(self.parent.write_projection(), |v| &mut v[index])
    }
}

impl<A, T> ValueProjection<T> for VecIndex<A, T>
where
    A: ReadProjection<Vec<T>>,
    T: Clone + 'static,
{
    fn value_projection(&self) -> T {
        let value = self.read_projection();
        (*value).clone()
    }
}

impl<A, T> ReadProjectionOpt<T> for VecGet<A, T>
where
    A: ReadProjection<Vec<T>>,
    T: 'static,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, T>>> {
        let index = self.index;
        self.parent
            .read_projection()
            .try_map(|r| Ref::filter_map(r, |items| items.get(index)).ok())
    }
}

impl<A, T> WriteProjectionOpt<T> for VecGet<A, T>
where
    A: WriteProjection<Vec<T>>,
    T: 'static,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, T, UnsyncStorage>> {
        let index = self.index;
        WriteLock::filter_map(self.parent.write_projection(), |items| items.get_mut(index))
    }
}

impl<A, T> ValueProjection<Option<T>> for VecGet<A, T>
where
    A: ReadProjection<Vec<T>>,
    T: Clone + 'static,
{
    fn value_projection(&self) -> Option<T> {
        self.parent.read_projection().get(self.index).cloned()
    }
}

impl<A, T> Optic<EachVec<A, T>>
where
    A: Clone + ReadProjection<Vec<T>>,
    T: 'static,
{
    /// Return the current vector length.
    pub fn len(&self) -> usize {
        let items: GenerationalRef<Ref<'static, Vec<T>>> = self.access.parent.read_projection();
        items.len()
    }

    /// Return `true` if the projected vector is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Borrow the child at `index` as an optional optics path.
    pub fn get(&self, index: usize) -> Optic<VecGet<A, T>, Optional> {
        Optic {
            access: VecGet {
                parent: self.access.parent.clone(),
                index,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Borrow the child at `index`, panicking if it is out of bounds.
    pub fn index(&self, index: usize) -> Optic<VecIndex<A, T>> {
        if index >= self.len() {
            panic!("index {index} out of bounds for optics vec projection");
        }
        Optic {
            access: VecIndex {
                parent: self.access.parent.clone(),
                index,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Iterate the child item signals.
    pub fn iter(&self) -> impl Iterator<Item = Optic<VecIndex<A, T>>> + '_ {
        let len = self.len();
        let parent = self.access.parent.clone();
        (0..len).map(move |index| Optic {
            access: VecIndex {
                parent: parent.clone(),
                index,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        })
    }
}

impl<A, T> Optic<EachVec<A, T>>
where
    A: WriteProjection<Vec<T>>,
    T: 'static,
{
    /// Push an item onto the end of the projected vector.
    pub fn push(&self, value: T) {
        let mut items: WriteLock<'static, Vec<T>, UnsyncStorage> =
            self.access.parent.write_projection();
        items.push(value);
    }

    /// Remove and return the item at `index`.
    pub fn remove(&self, index: usize) -> T {
        let mut items: WriteLock<'static, Vec<T>, UnsyncStorage> =
            self.access.parent.write_projection();
        items.remove(index)
    }

    /// Insert an item at `index`.
    pub fn insert(&self, index: usize, value: T) {
        let mut items: WriteLock<'static, Vec<T>, UnsyncStorage> =
            self.access.parent.write_projection();
        items.insert(index, value);
    }

    /// Clear all items from the projected vector.
    pub fn clear(&self) {
        let mut items: WriteLock<'static, Vec<T>, UnsyncStorage> =
            self.access.parent.write_projection();
        items.clear();
    }

    /// Retain only items that match `f`.
    pub fn retain(&self, f: impl FnMut(&T) -> bool) {
        let mut items: WriteLock<'static, Vec<T>, UnsyncStorage> =
            self.access.parent.write_projection();
        items.retain(f);
    }
}

/// Keyed iterable view over a projected `HashMap<K, V, S>`.
pub struct EachHashMap<A, K, V, S> {
    pub(crate) parent: A,
    pub(crate) _marker: PhantomData<fn() -> (K, V, S)>,
}

impl<A: Clone, K, V, S> Clone for EachHashMap<A, K, V, S> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            _marker: PhantomData,
        }
    }
}

/// Keyed child carrier inside [`EachHashMap`].
pub struct HashMapKey<A, K, V, S> {
    parent: A,
    key: K,
    _marker: PhantomData<fn() -> (V, S)>,
}

impl<A: Clone, K: Clone, V, S> Clone for HashMapKey<A, K, V, S> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            key: self.key.clone(),
            _marker: PhantomData,
        }
    }
}

/// Optional keyed child carrier inside [`EachHashMap`].
pub struct HashMapGet<A, K, V, S> {
    parent: A,
    key: K,
    _marker: PhantomData<fn() -> (V, S)>,
}

impl<A: Clone, K: Clone, V, S> Clone for HashMapGet<A, K, V, S> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            key: self.key.clone(),
            _marker: PhantomData,
        }
    }
}

impl<A, K, V, S> ReadProjection<V> for HashMapKey<A, K, V, S>
where
    A: ReadProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    fn read_projection(&self) -> GenerationalRef<Ref<'static, V>> {
        let key = &self.key;
        GenerationalRef::map(self.parent.read_projection(), |r| {
            Ref::map(r, |map| {
                map.get(key)
                    .unwrap_or_else(|| panic!("missing key in optics HashMap projection"))
            })
        })
    }
}

impl<A, K, V, S> WriteProjection<V> for HashMapKey<A, K, V, S>
where
    A: WriteProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    fn write_projection(&self) -> WriteLock<'static, V, UnsyncStorage> {
        let key = &self.key;
        WriteLock::map(self.parent.write_projection(), |map| {
            map.get_mut(key)
                .unwrap_or_else(|| panic!("missing key in optics HashMap projection"))
        })
    }
}

impl<A, K, V, S> ValueProjection<V> for HashMapKey<A, K, V, S>
where
    A: ReadProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: Clone + 'static,
    S: BuildHasher + 'static,
{
    fn value_projection(&self) -> V {
        let value = self.read_projection();
        (*value).clone()
    }
}

impl<A, K, V, S> ReadProjectionOpt<V> for HashMapGet<A, K, V, S>
where
    A: ReadProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, V>>> {
        let key = &self.key;
        self.parent
            .read_projection()
            .try_map(|r| Ref::filter_map(r, |map| map.get(key)).ok())
    }
}

impl<A, K, V, S> WriteProjectionOpt<V> for HashMapGet<A, K, V, S>
where
    A: WriteProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, V, UnsyncStorage>> {
        let key = &self.key;
        WriteLock::filter_map(self.parent.write_projection(), |map| map.get_mut(key))
    }
}

impl<A, K, V, S> ValueProjection<Option<V>> for HashMapGet<A, K, V, S>
where
    A: ReadProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: Clone + 'static,
    S: BuildHasher + 'static,
{
    fn value_projection(&self) -> Option<V> {
        self.parent.read_projection().get(&self.key).cloned()
    }
}

impl<A, K, V, S> Optic<EachHashMap<A, K, V, S>>
where
    A: Clone + ReadProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    /// Return the current map length.
    pub fn len(&self) -> usize {
        let map: GenerationalRef<Ref<'static, HashMap<K, V, S>>> =
            self.access.parent.read_projection();
        map.len()
    }

    /// Return `true` if the projected map is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return `true` if `key` exists in the projected map.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        let map: GenerationalRef<Ref<'static, HashMap<K, V, S>>> =
            self.access.parent.read_projection();
        map.contains_key(key)
    }

    /// Borrow the child at `key` as an optional optics path.
    pub fn get<Q>(&self, key: &Q) -> Optic<HashMapGet<A, K, V, S>, Optional>
    where
        Q: ?Sized + Hash + Eq + ToOwned<Owned = K>,
        K: Borrow<Q>,
    {
        Optic {
            access: HashMapGet {
                parent: self.access.parent.clone(),
                key: key.to_owned(),
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Borrow the child at `key`, panicking if it does not exist.
    pub fn get_unchecked(&self, key: K) -> Optic<HashMapKey<A, K, V, S>> {
        Optic {
            access: HashMapKey {
                parent: self.access.parent.clone(),
                key,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Iterate the map as `(key, child)` pairs.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (K, Optic<HashMapKey<A, K, V, S>>)> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let map: GenerationalRef<Ref<'static, HashMap<K, V, S>>> =
            self.access.parent.read_projection();
        let keys: Vec<K> = map.keys().cloned().collect();
        let parent = self.access.parent.clone();
        keys.into_iter().map(move |key| {
            let child = Optic {
                access: HashMapKey {
                    parent: parent.clone(),
                    key: key.clone(),
                    _marker: PhantomData,
                },
                _marker: PhantomData,
            };
            (key, child)
        })
    }

    /// Iterate the map values as child optics.
    pub fn values(
        &self,
    ) -> impl Iterator<Item = Optic<HashMapKey<A, K, V, S>>> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let map: GenerationalRef<Ref<'static, HashMap<K, V, S>>> =
            self.access.parent.read_projection();
        let keys: Vec<K> = map.keys().cloned().collect();
        let parent = self.access.parent.clone();
        keys.into_iter().map(move |key| Optic {
            access: HashMapKey {
                parent: parent.clone(),
                key,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        })
    }
}

impl<A, K, V, S> Optic<EachHashMap<A, K, V, S>>
where
    A: WriteProjection<HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    /// Insert `value` at `key`.
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let mut map: WriteLock<'static, HashMap<K, V, S>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.insert(key, value)
    }

    /// Remove the entry at `key`.
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        let mut map: WriteLock<'static, HashMap<K, V, S>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.remove(key)
    }

    /// Clear the projected map.
    pub fn clear(&self) {
        let mut map: WriteLock<'static, HashMap<K, V, S>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.clear();
    }

    /// Retain only entries matching `f`.
    pub fn retain(&self, f: impl FnMut(&K, &mut V) -> bool) {
        let mut map: WriteLock<'static, HashMap<K, V, S>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.retain(f);
    }
}

/// Keyed iterable view over a projected `BTreeMap<K, V>`.
pub struct EachBTreeMap<A, K, V> {
    pub(crate) parent: A,
    pub(crate) _marker: PhantomData<fn() -> (K, V)>,
}

impl<A: Clone, K, V> Clone for EachBTreeMap<A, K, V> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            _marker: PhantomData,
        }
    }
}

/// Keyed child carrier inside [`EachBTreeMap`].
pub struct BTreeMapKey<A, K, V> {
    parent: A,
    key: K,
    _marker: PhantomData<fn() -> V>,
}

impl<A: Clone, K: Clone, V> Clone for BTreeMapKey<A, K, V> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            key: self.key.clone(),
            _marker: PhantomData,
        }
    }
}

/// Optional keyed child carrier inside [`EachBTreeMap`].
pub struct BTreeMapGet<A, K, V> {
    parent: A,
    key: K,
    _marker: PhantomData<fn() -> V>,
}

impl<A: Clone, K: Clone, V> Clone for BTreeMapGet<A, K, V> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            key: self.key.clone(),
            _marker: PhantomData,
        }
    }
}

impl<A, K, V> ReadProjection<V> for BTreeMapKey<A, K, V>
where
    A: ReadProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    fn read_projection(&self) -> GenerationalRef<Ref<'static, V>> {
        let key = &self.key;
        GenerationalRef::map(self.parent.read_projection(), |r| {
            Ref::map(r, |map| {
                map.get(key)
                    .unwrap_or_else(|| panic!("missing key in optics BTreeMap projection"))
            })
        })
    }
}

impl<A, K, V> WriteProjection<V> for BTreeMapKey<A, K, V>
where
    A: WriteProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    fn write_projection(&self) -> WriteLock<'static, V, UnsyncStorage> {
        let key = &self.key;
        WriteLock::map(self.parent.write_projection(), |map| {
            map.get_mut(key)
                .unwrap_or_else(|| panic!("missing key in optics BTreeMap projection"))
        })
    }
}

impl<A, K, V> ValueProjection<V> for BTreeMapKey<A, K, V>
where
    A: ReadProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: Clone + 'static,
{
    fn value_projection(&self) -> V {
        let value = self.read_projection();
        (*value).clone()
    }
}

impl<A, K, V> ReadProjectionOpt<V> for BTreeMapGet<A, K, V>
where
    A: ReadProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, V>>> {
        let key = &self.key;
        self.parent
            .read_projection()
            .try_map(|r| Ref::filter_map(r, |map| map.get(key)).ok())
    }
}

impl<A, K, V> WriteProjectionOpt<V> for BTreeMapGet<A, K, V>
where
    A: WriteProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, V, UnsyncStorage>> {
        let key = &self.key;
        WriteLock::filter_map(self.parent.write_projection(), |map| map.get_mut(key))
    }
}

impl<A, K, V> ValueProjection<Option<V>> for BTreeMapGet<A, K, V>
where
    A: ReadProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: Clone + 'static,
{
    fn value_projection(&self) -> Option<V> {
        self.parent.read_projection().get(&self.key).cloned()
    }
}

impl<A, K, V> Optic<EachBTreeMap<A, K, V>>
where
    A: Clone + ReadProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    /// Return the current map length.
    pub fn len(&self) -> usize {
        let map: GenerationalRef<Ref<'static, BTreeMap<K, V>>> =
            self.access.parent.read_projection();
        map.len()
    }

    /// Return `true` if the projected map is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return `true` if `key` exists in the projected map.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Ord,
        K: Borrow<Q>,
    {
        let map: GenerationalRef<Ref<'static, BTreeMap<K, V>>> =
            self.access.parent.read_projection();
        map.contains_key(key)
    }

    /// Borrow the child at `key` as an optional optics path.
    pub fn get<Q>(&self, key: &Q) -> Optic<BTreeMapGet<A, K, V>, Optional>
    where
        Q: ?Sized + Ord + ToOwned<Owned = K>,
        K: Borrow<Q>,
    {
        Optic {
            access: BTreeMapGet {
                parent: self.access.parent.clone(),
                key: key.to_owned(),
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Borrow the child at `key`, panicking if it does not exist.
    pub fn get_unchecked(&self, key: K) -> Optic<BTreeMapKey<A, K, V>> {
        Optic {
            access: BTreeMapKey {
                parent: self.access.parent.clone(),
                key,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Iterate the map as `(key, child)` pairs.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (K, Optic<BTreeMapKey<A, K, V>>)> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let map: GenerationalRef<Ref<'static, BTreeMap<K, V>>> =
            self.access.parent.read_projection();
        let keys: Vec<K> = map.keys().cloned().collect();
        let parent = self.access.parent.clone();
        keys.into_iter().map(move |key| {
            let child = Optic {
                access: BTreeMapKey {
                    parent: parent.clone(),
                    key: key.clone(),
                    _marker: PhantomData,
                },
                _marker: PhantomData,
            };
            (key, child)
        })
    }

    /// Iterate the map values as child optics.
    pub fn values(
        &self,
    ) -> impl Iterator<Item = Optic<BTreeMapKey<A, K, V>>> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let map: GenerationalRef<Ref<'static, BTreeMap<K, V>>> =
            self.access.parent.read_projection();
        let keys: Vec<K> = map.keys().cloned().collect();
        let parent = self.access.parent.clone();
        keys.into_iter().map(move |key| Optic {
            access: BTreeMapKey {
                parent: parent.clone(),
                key,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        })
    }
}

impl<A, K, V> Optic<EachBTreeMap<A, K, V>>
where
    A: WriteProjection<BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    /// Insert `value` at `key`.
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let mut map: WriteLock<'static, BTreeMap<K, V>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.insert(key, value)
    }

    /// Remove the entry at `key`.
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Ord,
        K: Borrow<Q>,
    {
        let mut map: WriteLock<'static, BTreeMap<K, V>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.remove(key)
    }

    /// Clear the projected map.
    pub fn clear(&self) {
        let mut map: WriteLock<'static, BTreeMap<K, V>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.clear();
    }

    /// Retain only entries matching `f`.
    pub fn retain(&self, f: impl FnMut(&K, &mut V) -> bool) {
        let mut map: WriteLock<'static, BTreeMap<K, V>, UnsyncStorage> =
            self.access.parent.write_projection();
        map.retain(f);
    }
}
