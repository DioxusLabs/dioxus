//! Unified `.iter()` entry point that picks the right collection carrier.
//!
//! Replaces the per-shape entry methods (`.each()`, `.each_hash_map()`,
//! `.each_btree_map()`) with a single `.iter()` callable directly on any
//! [`Access`] carrier whose `Target` is `Vec<T>`, `HashMap<K, V, S>`, or
//! `BTreeMap<K, V>`. Dispatch happens through the sealed [`IterShape`] trait
//! (a GAT named `Each<A>` picks the matching carrier), so a single blanket
//! `impl OpticIter for A: Access<A::Target: IterShape>` covers every shape
//! without overlapping impls.
//!
//! The returned [`Optic`] over the matching carrier ([`EachVec`] /
//! [`EachHashMap`] / [`EachBTreeMap`]) is reusable: it owns no iterator
//! state and can be borrowed any number of times to produce a fresh
//! `Iterator` via [`IntoIterator`] for `&Optic<...>`.

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

use crate::collection::{BTreeMapKey, EachBTreeMap, EachHashMap, EachVec, HashMapKey, VecIndex};
use crate::combinator::Access;
use crate::signal::{Optic, Required};

mod sealed {
    pub trait Sealed {}
}

/// Sealed shape-dispatch trait used to pick the iteration carrier for a
/// collection target. One impl per supported container type.
pub trait IterShape: sealed::Sealed {
    /// The iteration carrier for this shape (e.g. [`EachVec`]).
    type Each<A>;

    /// Build the iteration carrier from a parent accessor.
    fn build_each<A>(parent: A) -> Self::Each<A>;
}

impl<T: 'static> sealed::Sealed for Vec<T> {}
impl<T: 'static> IterShape for Vec<T> {
    type Each<A> = EachVec<A, T>;
    fn build_each<A>(parent: A) -> EachVec<A, T> {
        EachVec {
            parent,
            _marker: PhantomData,
        }
    }
}

impl<K, V, S> sealed::Sealed for HashMap<K, V, S>
where
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
}
impl<K, V, S> IterShape for HashMap<K, V, S>
where
    K: Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    type Each<A> = EachHashMap<A, K, V, S>;
    fn build_each<A>(parent: A) -> EachHashMap<A, K, V, S> {
        EachHashMap {
            parent,
            _marker: PhantomData,
        }
    }
}

impl<K, V> sealed::Sealed for BTreeMap<K, V>
where
    K: Ord + 'static,
    V: 'static,
{
}
impl<K, V> IterShape for BTreeMap<K, V>
where
    K: Ord + 'static,
    V: 'static,
{
    type Each<A> = EachBTreeMap<A, K, V>;
    fn build_each<A>(parent: A) -> EachBTreeMap<A, K, V> {
        EachBTreeMap {
            parent,
            _marker: PhantomData,
        }
    }
}

/// Carrier-agnostic `.iter()` entry point.
///
/// Implemented for any [`Access`] whose `Target` implements [`IterShape`]
/// (i.e. `Vec`, `HashMap`, or `BTreeMap`). Returns the matching
/// `Optic<Each*<...>>` carrier so existing inherent methods (`.len()`,
/// `.push()`, `.insert()`, `.values()`, `.iter()`, etc.) work directly on
/// the result.
pub trait OpticIter: Access + Sized {
    /// Step into the collection. Subsequent borrows of the returned optic
    /// can produce a fresh iterator any number of times via
    /// [`IntoIterator`] for `&Optic<...>`.
    fn iter(self) -> Optic<<Self::Target as IterShape>::Each<Self>, Required>
    where
        Self::Target: IterShape,
    {
        Optic {
            access: <Self::Target as IterShape>::build_each::<Self>(self),
            _marker: PhantomData,
        }
    }
}

impl<A: Access> OpticIter for A {}

// ============================================================================
// IntoIterator impls so `for x in &signal.iter() { ... }` works repeatedly.
// Each call produces a fresh iterator that snapshots the collection's
// current shape (length / keys), matching the existing inherent
// `Optic<Each*<...>>::iter` semantics.
// ============================================================================

impl<'a, A, T> IntoIterator for &'a Optic<EachVec<A, T>, Required>
where
    A: Clone + Access<Target = Vec<T>>,
    T: 'static,
{
    type Item = Optic<VecIndex<A, T>, Required>;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        let len = self
            .access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .len();
        let parent = self.access.parent.clone();
        Box::new((0..len).map(move |index| Optic {
            access: VecIndex {
                parent: parent.clone(),
                index,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }))
    }
}

impl<'a, A, K, V, S> IntoIterator for &'a Optic<EachHashMap<A, K, V, S>, Required>
where
    A: Clone + Access<Target = HashMap<K, V, S>>,
    K: Clone + Eq + Hash + 'static,
    V: 'static,
    S: BuildHasher + 'static,
{
    type Item = (K, Optic<HashMapKey<A, K, V, S>, Required>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        let keys: Vec<K> = self
            .access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .keys()
            .cloned()
            .collect();
        let parent = self.access.parent.clone();
        Box::new(keys.into_iter().map(move |key| {
            let child = Optic {
                access: HashMapKey {
                    parent: parent.clone(),
                    key: key.clone(),
                    _marker: PhantomData,
                },
                _marker: PhantomData,
            };
            (key, child)
        }))
    }
}

impl<'a, A, K, V> IntoIterator for &'a Optic<EachBTreeMap<A, K, V>, Required>
where
    A: Clone + Access<Target = BTreeMap<K, V>>,
    K: Clone + Ord + 'static,
    V: 'static,
{
    type Item = (K, Optic<BTreeMapKey<A, K, V>, Required>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        let keys: Vec<K> = self
            .access
            .parent
            .try_read()
            .expect("optics: collection parent path produced no value")
            .keys()
            .cloned()
            .collect();
        let parent = self.access.parent.clone();
        Box::new(keys.into_iter().map(move |key| {
            let child = Optic {
                access: BTreeMapKey {
                    parent: parent.clone(),
                    key: key.clone(),
                    _marker: PhantomData,
                },
                _marker: PhantomData,
            };
            (key, child)
        }))
    }
}
