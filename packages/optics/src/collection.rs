use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashMap},
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

use generational_box::{AnyStorage, WriteLock};

use crate::{
    combinator::{Access, AccessMut, Combinator, Resolve, ValueAccess},
    path::{PathBuffer, PathSegment, Pathed},
    signal::Optic,
};

/// Collection/key lookup projection that always returns an optional child path.
pub trait GetProjection<Key> {
    /// Child carrier produced by the lookup.
    type Child;

    /// Project the child at `key`.
    fn get_projection(&self, key: Key) -> Self::Child;
}

/// Flatten `Option<Option<X>>` into `Option<X>`.
#[derive(Clone, Copy, Default)]
pub struct FlattenSomeOp;

/// Carrier alias for a single `Option` flattening step.
pub type FlattenSome<A> = Combinator<A, FlattenSomeOp>;

impl<X> Resolve<FlattenSomeOp> for Option<X> {
    type Input = Option<Option<X>>;
    fn resolve(input: Self::Input, _: &FlattenSomeOp) -> Self {
        input.flatten()
    }
}

// ============================================================================
// Vec
// ============================================================================

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
///
/// `try_read` returns `None` when the index is out of bounds. Calling `read()`
/// on a `Required`-tagged optic over this carrier panics on out-of-bounds.
pub struct VecIndex<A, T> {
    pub(crate) parent: A,
    pub(crate) index: usize,
    pub(crate) _marker: PhantomData<fn() -> T>,
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

impl<A, T> Access for VecIndex<A, T>
where
    A: Access<Target = Vec<T>>,
    T: 'static,
{
    type Target = T;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, T>> {
        let index = self.index;
        self.parent
            .try_read()
            .and_then(|r| A::Storage::try_map(r, move |v| v.get(index)))
    }

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, T>> {
        let index = self.index;
        self.parent
            .try_peek()
            .and_then(|r| A::Storage::try_map(r, move |v| v.get(index)))
    }
}

impl<A, T> AccessMut for VecIndex<A, T>
where
    A: AccessMut<Target = Vec<T>>,
    T: 'static,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, T, A::Storage, A::WriteMetadata>> {
        let index = self.index;
        self.parent
            .try_write()
            .and_then(|w| WriteLock::filter_map(w, move |v| v.get_mut(index)))
    }
}

impl<A, T> ValueAccess<T> for VecIndex<A, T>
where
    A: Access<Target = Vec<T>>,
    T: Clone + 'static,
{
    fn value(&self) -> T {
        self.try_read()
            .expect("optics: Vec index out of bounds")
            .clone()
    }
}

impl<A, T> ValueAccess<Option<T>> for VecIndex<A, T>
where
    A: Access<Target = Vec<T>>,
    T: Clone + 'static,
{
    fn value(&self) -> Option<T> {
        self.try_read().as_deref().cloned()
    }
}

impl<A, T> Pathed for VecIndex<A, T>
where
    A: Pathed,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
        sink.push(PathSegment::index(self.index as u64));
    }
}

impl<A, T> Optic<EachVec<A, T>>
where
    A: Clone + Access<Target = Vec<T>>,
    T: 'static,
{
    /// Return the current vector length.
    pub fn len(&self) -> usize {
        self.access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .len()
    }

    /// Return `true` if the projected vector is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Borrow the child at `index` as a Required optic. Panics on
    /// out-of-bounds access through `read`/`write`.
    pub fn index(&self, index: usize) -> Optic<VecIndex<A, T>> {
        Optic {
            access: VecIndex {
                parent: self.access.parent.clone(),
                index,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Iterate the child item optics.
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
    A: AccessMut<Target = Vec<T>>,
    T: 'static,
{
    pub fn push(&self, value: T) {
        let mut items = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        items.push(value);
    }

    pub fn remove(&self, index: usize) -> T {
        let mut items = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        items.remove(index)
    }

    pub fn insert(&self, index: usize, value: T) {
        let mut items = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        items.insert(index, value);
    }

    pub fn clear(&self) {
        let mut items = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        items.clear();
    }

    pub fn retain(&self, f: impl FnMut(&T) -> bool) {
        let mut items = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        items.retain(f);
    }
}

// ============================================================================
// HashMap
// ============================================================================

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
///
/// `try_read` returns `None` when the key is absent from the map.
pub struct HashMapKey<A, K, V, S> {
    pub(crate) parent: A,
    pub(crate) key: K,
    pub(crate) _marker: PhantomData<fn() -> (V, S)>,
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

impl<A, K, V, S> Access for HashMapKey<A, K, V, S>
where
    A: Access<Target = HashMap<K, V, S>>,
    K: Clone + Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    type Target = V;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, V>> {
        let key = self.key.clone();
        self.parent
            .try_read()
            .and_then(|r| A::Storage::try_map(r, move |map| map.get(&key)))
    }

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, V>> {
        let key = self.key.clone();
        self.parent
            .try_peek()
            .and_then(|r| A::Storage::try_map(r, move |map| map.get(&key)))
    }
}

impl<A, K, V, S> AccessMut for HashMapKey<A, K, V, S>
where
    A: AccessMut<Target = HashMap<K, V, S>>,
    K: Clone + Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, V, A::Storage, A::WriteMetadata>> {
        let key = self.key.clone();
        self.parent
            .try_write()
            .and_then(|w| WriteLock::filter_map(w, move |map| map.get_mut(&key)))
    }
}

impl<A, K, V, S> ValueAccess<V> for HashMapKey<A, K, V, S>
where
    A: Access<Target = HashMap<K, V, S>>,
    K: Clone + Eq + Hash + 'static,
    V: Clone + 'static,
    S: BuildHasher + 'static,
{
    fn value(&self) -> V {
        self.try_read()
            .expect("optics: missing key in HashMap projection")
            .clone()
    }
}

impl<A, K, V, S> ValueAccess<Option<V>> for HashMapKey<A, K, V, S>
where
    A: Access<Target = HashMap<K, V, S>>,
    K: Clone + Eq + Hash + 'static,
    V: Clone + 'static,
    S: BuildHasher + 'static,
{
    fn value(&self) -> Option<V> {
        self.try_read().as_deref().cloned()
    }
}

impl<A, K, V, S> Pathed for HashMapKey<A, K, V, S>
where
    A: Pathed,
    K: Hash,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
        sink.push(PathSegment::hashed(&self.key));
    }
}

impl<A, K, V, S> Optic<EachHashMap<A, K, V, S>>
where
    A: Clone + Access<Target = HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    pub fn len(&self) -> usize {
        self.access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .contains_key(key)
    }

    /// Borrow the child at `key` as a Required optic. Panics on missing key
    /// through `read`/`write`.
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

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (K, Optic<HashMapKey<A, K, V, S>>)> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let keys: Vec<K> = {
            let map = self
                .access
                .parent
                .try_read()
                .expect("optics: collection parent path produced no value");
            map.keys().cloned().collect()
        };
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

    pub fn values(
        &self,
    ) -> impl Iterator<Item = Optic<HashMapKey<A, K, V, S>>> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let keys: Vec<K> = {
            let map = self
                .access
                .parent
                .try_read()
                .expect("optics: collection parent path produced no value");
            map.keys().cloned().collect()
        };
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
    A: AccessMut<Target = HashMap<K, V, S>>,
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.insert(key, value)
    }

    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.remove(key)
    }

    pub fn clear(&self) {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.clear();
    }

    pub fn retain(&self, f: impl FnMut(&K, &mut V) -> bool) {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.retain(f);
    }
}

// ============================================================================
// BTreeMap
// ============================================================================

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
    pub(crate) parent: A,
    pub(crate) key: K,
    pub(crate) _marker: PhantomData<fn() -> V>,
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

impl<A, K, V> Access for BTreeMapKey<A, K, V>
where
    A: Access<Target = BTreeMap<K, V>>,
    K: Clone + Ord + 'static,
    V: 'static,
{
    type Target = V;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, V>> {
        let key = self.key.clone();
        self.parent
            .try_read()
            .and_then(|r| A::Storage::try_map(r, move |map| map.get(&key)))
    }

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, V>> {
        let key = self.key.clone();
        self.parent
            .try_peek()
            .and_then(|r| A::Storage::try_map(r, move |map| map.get(&key)))
    }
}

impl<A, K, V> AccessMut for BTreeMapKey<A, K, V>
where
    A: AccessMut<Target = BTreeMap<K, V>>,
    K: Clone + Ord + 'static,
    V: 'static,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, V, A::Storage, A::WriteMetadata>> {
        let key = self.key.clone();
        self.parent
            .try_write()
            .and_then(|w| WriteLock::filter_map(w, move |map| map.get_mut(&key)))
    }
}

impl<A, K, V> ValueAccess<V> for BTreeMapKey<A, K, V>
where
    A: Access<Target = BTreeMap<K, V>>,
    K: Clone + Ord + 'static,
    V: Clone + 'static,
{
    fn value(&self) -> V {
        self.try_read()
            .expect("optics: missing key in BTreeMap projection")
            .clone()
    }
}

impl<A, K, V> ValueAccess<Option<V>> for BTreeMapKey<A, K, V>
where
    A: Access<Target = BTreeMap<K, V>>,
    K: Clone + Ord + 'static,
    V: Clone + 'static,
{
    fn value(&self) -> Option<V> {
        self.try_read().as_deref().cloned()
    }
}

impl<A, K, V> Pathed for BTreeMapKey<A, K, V>
where
    A: Pathed,
    K: Hash,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
        sink.push(PathSegment::hashed(&self.key));
    }
}

impl<A, K, V> Optic<EachBTreeMap<A, K, V>>
where
    A: Clone + Access<Target = BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    pub fn len(&self) -> usize {
        self.access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Ord,
        K: Borrow<Q>,
    {
        self.access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .contains_key(key)
    }

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

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (K, Optic<BTreeMapKey<A, K, V>>)> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let keys: Vec<K> = {
            let map = self
                .access
                .parent
                .try_read()
                .expect("optics: collection parent path produced no value");
            map.keys().cloned().collect()
        };
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

    pub fn values(
        &self,
    ) -> impl Iterator<Item = Optic<BTreeMapKey<A, K, V>>> + DoubleEndedIterator + '_
    where
        K: Clone,
    {
        let keys: Vec<K> = {
            let map = self
                .access
                .parent
                .try_read()
                .expect("optics: collection parent path produced no value");
            map.keys().cloned().collect()
        };
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
    A: AccessMut<Target = BTreeMap<K, V>>,
    K: Ord + 'static,
    V: 'static,
{
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.insert(key, value)
    }

    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Ord,
        K: Borrow<Q>,
    {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.remove(key)
    }

    pub fn clear(&self) {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.clear();
    }

    pub fn retain(&self, f: impl FnMut(&K, &mut V) -> bool) {
        let mut map = self
            .access
            .parent
            .try_write()
            .expect("optics: collection parent path produced no value");
        map.retain(f);
    }
}

// ============================================================================
// GetProjection: .get(key) on each collection view
// ============================================================================

impl<A, T> GetProjection<usize> for EachVec<A, T>
where
    A: Clone,
{
    type Child = VecIndex<A, T>;

    fn get_projection(&self, index: usize) -> Self::Child {
        VecIndex {
            parent: self.parent.clone(),
            index,
            _marker: PhantomData,
        }
    }
}

/// Nested `.get(idx)` on a `VecIndex<_, Vec<T>>` — chains through the
/// already-optional parent.
impl<A, T> GetProjection<usize> for VecIndex<A, Vec<T>>
where
    A: Clone + Access<Target = Vec<Vec<T>>>,
    T: 'static,
{
    type Child = VecIndex<Self, T>;

    fn get_projection(&self, index: usize) -> Self::Child {
        VecIndex {
            parent: self.clone(),
            index,
            _marker: PhantomData,
        }
    }
}

impl<'a, A, K, V, S, Q> GetProjection<&'a Q> for EachHashMap<A, K, V, S>
where
    A: Clone,
    Q: ?Sized + Hash + Eq + ToOwned<Owned = K>,
{
    type Child = HashMapKey<A, K, V, S>;

    fn get_projection(&self, key: &'a Q) -> Self::Child {
        HashMapKey {
            parent: self.parent.clone(),
            key: key.to_owned(),
            _marker: PhantomData,
        }
    }
}

/// Nested `.get(key)` on a `HashMapKey<_, HashMap<K2, V2, S2>>`.
impl<'a, A, KOuter, K, V, SOuter, S, Q> GetProjection<&'a Q>
    for HashMapKey<A, KOuter, HashMap<K, V, S>, SOuter>
where
    A: Clone + Access<Target = HashMap<KOuter, HashMap<K, V, S>, SOuter>>,
    KOuter: Clone + Eq + Hash + 'static,
    Q: ?Sized + Hash + Eq + ToOwned<Owned = K>,
    K: Eq + Hash + 'static,
    V: 'static,
    SOuter: BuildHasher + 'static,
    S: BuildHasher + 'static,
{
    type Child = HashMapKey<Self, K, V, S>;

    fn get_projection(&self, key: &'a Q) -> Self::Child {
        HashMapKey {
            parent: self.clone(),
            key: key.to_owned(),
            _marker: PhantomData,
        }
    }
}

impl<'a, A, K, V, Q> GetProjection<&'a Q> for EachBTreeMap<A, K, V>
where
    A: Clone,
    Q: ?Sized + Ord + ToOwned<Owned = K>,
{
    type Child = BTreeMapKey<A, K, V>;

    fn get_projection(&self, key: &'a Q) -> Self::Child {
        BTreeMapKey {
            parent: self.parent.clone(),
            key: key.to_owned(),
            _marker: PhantomData,
        }
    }
}

impl<'a, A, KOuter, K, V, Q> GetProjection<&'a Q> for BTreeMapKey<A, KOuter, BTreeMap<K, V>>
where
    A: Clone + Access<Target = BTreeMap<KOuter, BTreeMap<K, V>>>,
    KOuter: Clone + Ord + 'static,
    Q: ?Sized + Ord + ToOwned<Owned = K>,
    K: Ord + 'static,
    V: 'static,
{
    type Child = BTreeMapKey<Self, K, V>;

    fn get_projection(&self, key: &'a Q) -> Self::Child {
        BTreeMapKey {
            parent: self.clone(),
            key: key.to_owned(),
            _marker: PhantomData,
        }
    }
}

// The FlattenSomeOp Resolve impl lives in resource.rs; re-assert a no-op
// Access impl here so flatten_some still composes on the ref channel.
impl<A, X: 'static> Access for Combinator<A, FlattenSomeOp>
where
    A: Access<Target = Option<X>>,
{
    type Target = X;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, X>> {
        self.parent
            .try_read()
            .and_then(|r| A::Storage::try_map(r, |o| o.as_ref()))
    }

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, X>> {
        self.parent
            .try_peek()
            .and_then(|r| A::Storage::try_map(r, |o| o.as_ref()))
    }
}

impl<A, X: 'static> AccessMut for Combinator<A, FlattenSomeOp>
where
    A: AccessMut<Target = Option<X>>,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, X, A::Storage, A::WriteMetadata>> {
        self.parent
            .try_write()
            .and_then(|w| WriteLock::filter_map(w, |o| o.as_mut()))
    }
}

// `flatten_some` collapses `Option<Option<X>>` into `Option<X>` — it doesn't
// narrow the data, so its subscription path is identical to the parent's.
// No segment is added.
impl<A> Pathed for Combinator<A, FlattenSomeOp>
where
    A: Pathed,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
    }
}

// Silence unused import warnings when the Access impl above isn't needed.
#[allow(dead_code)]
fn _touch_resolve<A, Op, T>()
where
    Combinator<A, Op>: Access<Target = T>,
    T: Resolve<Op>,
{
}
