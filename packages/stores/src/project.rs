//! Generic projection primitives.
//!
//! [`ProjectMap`] is the functional carrier/lens mapping layer.
//! [`ProjectScope`] adds reactive path scoping, subscription tracking, and
//! dirty marking on top.
//! [`Project`] is the convenience umbrella for projector carriers that do
//! both. It's implemented by:
//!
//! - [`crate::scope::SelectorScope`] — for stores; records path keys in the
//!   subscription tree so writes only notify interested subscribers.
//! - [`LensOnly`] — for resources (and any other lens-only view); ignores path
//!   keys entirely, paying no subscription/tracking cost.
//! - [`dioxus_signals::Signal`], [`dioxus_signals::ReadSignal`], and
//!   [`dioxus_signals::WriteSignal`] — pathless roots that project through
//!   mapped lenses while subscribing at whole-signal granularity.
//!
//! Shape-specific projection methods (`transpose` / `ok` / `err` / `index` /
//! `deref` / …) are defined as default methods on traits bounded by
//! [`ProjectMap`] or [`ProjectScope`], so adding a new shape adds code once
//! and it works on both `Store` and resource-backed types automatically.

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};
use std::iter::FusedIterator;
use std::ops::{Index, IndexMut};
use std::panic::Location;

use crate::impls::btreemap::GetWrite as BTreeMapGetWrite;
use crate::impls::hashmap::GetWrite as HashMapGetWrite;
use crate::impls::index::{IndexSelector, IndexWrite};
use crate::scope::SelectorScope;
use crate::subscriptions::PathKey;
use dioxus_core::{ReactiveContext, Subscribers};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, BoxedSignalStorage, MappedMutSignal, ReadSignal,
    Readable, ReadableExt, ReadableRef, Signal, Writable, WritableExt, WritableRef, WriteSignal,
};

// ---------------------------------------------------------------------------
// Core traits
// ---------------------------------------------------------------------------

/// The lens produced by mapping a projector through `F` / `FMut`.
#[allow(type_alias_bounds)]
pub type MappedProjectLens<P: ProjectMap, U: ?Sized, F, FMut> =
    MappedMutSignal<U, <P as ProjectMap>::Lens, F, FMut>;

/// Rebind a projector carrier to a mapped child lens.
#[allow(type_alias_bounds)]
pub type Projected<P: ProjectMap, U: ?Sized, F, FMut> =
    <P as ProjectMap>::Rebind<U, MappedProjectLens<P, U, F, FMut>>;

/// Abstracts over "same projection semantics, different lens in the same storage".
pub trait ProjectMap: Sized {
    /// The lens this projection reads/writes through.
    type Lens: Readable;

    /// Rebind this carrier to a different lens/target pair while preserving
    /// its projection semantics.
    type Rebind<U: ?Sized + 'static, L>: Project<
        Lens: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage>,
    >
    where
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

    /// Rebuild this carrier around a derived lens built from the current one.
    fn project_compose<U, L>(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>
    where
        U: ?Sized + 'static,
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

    /// Map the lens without introducing a path-level child (used for things
    /// like `Deref` / `as_slice` that project to a different view of the same
    /// cell without new subscription granularity).
    fn project_map<U, F, FMut>(self, map: F, map_mut: FMut) -> Projected<Self, U, F, FMut>
    where
        U: ?Sized + 'static,
        <Self::Lens as Readable>::Target: 'static,
        F: Fn(&<Self::Lens as Readable>::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut <Self::Lens as Readable>::Target) -> &mut U + 'static + Copy,
    {
        self.project_compose::<U, _>(|lens| MappedMutSignal::new(lens, map, map_mut))
    }

    /// Borrow the underlying lens.
    fn project_lens(&self) -> &Self::Lens;

    /// Write to the lens without any tracking/notifying (fine-grained
    /// dirty-marking is done by `project_mark_dirty*`).
    fn project_write_untracked(&self) -> WritableRef<'static, Self::Lens>
    where
        Self::Lens: Writable,
        <Self::Lens as Readable>::Target: 'static;

    /// Read the current value without tracking.
    fn project_peek(&self) -> ReadableRef<'static, Self::Lens>
    where
        <Self::Lens as Readable>::Target: 'static,
    {
        self.project_lens().peek_unchecked()
    }
}

/// Reactive path scoping, subscription tracking, and dirty marking for a
/// projector carrier.
pub trait ProjectScope: ProjectMap {
    /// Scope this projection to a keyed child without changing the lens type.
    fn project_key(self, key: PathKey) -> Self;

    /// Scope this projection to a hashed child without changing the lens type.
    fn project_hash_key<K: Hash + ?Sized>(self, key: &K) -> Self;

    /// Scope into a keyed child, wrapping the lens with a new map/map_mut.
    /// `SelectorScope` interprets the key as a position in the subscription
    /// tree; `LensOnly` ignores it.
    fn project_child<U, F, FMut>(
        self,
        key: PathKey,
        map: F,
        map_mut: FMut,
    ) -> Projected<Self, U, F, FMut>
    where
        U: ?Sized + 'static,
        <Self::Lens as Readable>::Target: 'static,
        F: Fn(&<Self::Lens as Readable>::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut <Self::Lens as Readable>::Target) -> &mut U + 'static + Copy,
    {
        self.project_key(key).project_map(map, map_mut)
    }

    /// Track this projection's scope shallowly (no-op for LensOnly).
    fn project_track_shallow(&self);

    /// Track this projection recursively (no-op for LensOnly).
    fn project_track(&self);

    /// Mark this scope as dirty at the root path (no-op for LensOnly — the
    /// signal write itself notifies its subscribers).
    fn project_mark_dirty(&self);

    /// Mark this scope shallow-dirty (length/structure changed but not every element).
    fn project_mark_dirty_shallow(&self);

    /// Mark elements at and after `index` dirty (for sequence structures).
    fn project_mark_dirty_at_and_after_index(&self, index: usize);

    /// Mark the child at a hashed key dirty (for map-like structures).
    /// No-op for LensOnly; SelectorScope hashes the key and marks the child path.
    fn project_mark_hash_child_dirty<K: Hash + ?Sized>(&self, key: &K);
}

/// Convenience umbrella for carriers that support both lens mapping and
/// reactive path scoping.
pub trait Project: ProjectScope {}

impl<T> Project for T where T: ProjectScope {}

// ---------------------------------------------------------------------------
// SelectorScope impl — tracked
// ---------------------------------------------------------------------------

impl<Lens> ProjectMap for SelectorScope<Lens>
where
    Lens: Readable,
    Lens::Target: 'static,
{
    type Lens = Lens;

    type Rebind<U: ?Sized + 'static, L>
        = SelectorScope<L>
    where
        L: Readable<Target = U, Storage = Lens::Storage> + 'static;

    fn project_compose<U, L>(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>
    where
        U: ?Sized + 'static,
        L: Readable<Target = U, Storage = Lens::Storage> + 'static,
    {
        self.map_writer(map)
    }

    fn project_lens(&self) -> &Lens {
        self.writer()
    }

    fn project_write_untracked(&self) -> WritableRef<'static, Lens>
    where
        Lens: Writable,
        Lens::Target: 'static,
    {
        self.write_untracked()
    }
}

impl<Lens> ProjectScope for SelectorScope<Lens>
where
    Lens: Readable,
    Lens::Target: 'static,
{
    fn project_key(self, key: PathKey) -> Self {
        self.child_unmapped(key)
    }

    fn project_hash_key<K: Hash + ?Sized>(self, key: &K) -> Self {
        self.hash_child_unmapped(key)
    }

    fn project_track_shallow(&self) {
        SelectorScope::track_shallow(self);
    }

    fn project_track(&self) {
        SelectorScope::track(self);
    }

    fn project_mark_dirty(&self) {
        SelectorScope::mark_dirty(self);
    }

    fn project_mark_dirty_shallow(&self) {
        SelectorScope::mark_dirty_shallow(self);
    }

    fn project_mark_dirty_at_and_after_index(&self, index: usize) {
        SelectorScope::mark_dirty_at_and_after_index(self, index);
    }

    fn project_mark_hash_child_dirty<K: std::hash::Hash + ?Sized>(&self, key: &K) {
        let child = self.as_ref().hash_child_unmapped(key);
        SelectorScope::mark_dirty(&child);
    }
}

// ---------------------------------------------------------------------------
// Store — delegates to its internal SelectorScope. This is what lets every
// shape trait (ProjectOption, ProjectResult, …) work on `Store` directly,
// with no per-method shims.
// ---------------------------------------------------------------------------

use crate::store::Store;

impl<T, Lens> ProjectMap for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T> + 'static,
{
    type Lens = Lens;

    type Rebind<U: ?Sized + 'static, L>
        = Store<U, L>
    where
        L: Readable<Target = U, Storage = Lens::Storage> + 'static;

    fn project_compose<U, L>(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>
    where
        U: ?Sized + 'static,
        L: Readable<Target = U, Storage = Lens::Storage> + 'static,
    {
        self.into_selector().project_compose(map).into()
    }

    fn project_lens(&self) -> &Lens {
        self.selector().project_lens()
    }

    fn project_write_untracked(&self) -> WritableRef<'static, Lens>
    where
        Lens: Writable,
        Lens::Target: 'static,
    {
        self.selector().project_write_untracked()
    }
}

impl<T, Lens> ProjectScope for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T> + 'static,
{
    fn project_key(self, key: PathKey) -> Self {
        self.into_selector().project_key(key).into()
    }

    fn project_hash_key<K: Hash + ?Sized>(self, key: &K) -> Self {
        self.into_selector().project_hash_key(key).into()
    }

    fn project_track_shallow(&self) {
        self.selector().project_track_shallow();
    }

    fn project_track(&self) {
        self.selector().project_track();
    }

    fn project_mark_dirty(&self) {
        self.selector().project_mark_dirty();
    }

    fn project_mark_dirty_shallow(&self) {
        self.selector().project_mark_dirty_shallow();
    }

    fn project_mark_dirty_at_and_after_index(&self, index: usize) {
        self.selector().project_mark_dirty_at_and_after_index(index);
    }

    fn project_mark_hash_child_dirty<K: std::hash::Hash + ?Sized>(&self, key: &K) {
        self.selector().project_mark_hash_child_dirty(key);
    }
}

// ---------------------------------------------------------------------------
// LensOnly — untracked lens projection, used by resources
// ---------------------------------------------------------------------------

/// A lens wrapped for projection without a subscription tree. Used by
/// resources: the underlying signal already notifies subscribers on change, so
/// per-path tracking is pure overhead for this case.
pub struct LensOnly<L> {
    lens: L,
}

impl<L: Copy> Copy for LensOnly<L> {}
impl<L: Clone> Clone for LensOnly<L> {
    fn clone(&self) -> Self {
        LensOnly {
            lens: self.lens.clone(),
        }
    }
}

impl<L> LensOnly<L> {
    /// Wrap a lens.
    pub fn new(lens: L) -> Self {
        LensOnly { lens }
    }

    /// Unwrap into the inner lens.
    pub fn into_inner(self) -> L {
        self.lens
    }
}

impl<L: Readable> Readable for LensOnly<L> {
    type Target = L::Target;
    type Storage = L::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.lens.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.lens.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        Self::Target: 'static,
    {
        self.lens.subscribers()
    }
}

impl<L: Writable> Writable for LensOnly<L> {
    type WriteMetadata = L::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.lens.try_write_unchecked()
    }
}

impl<L> std::fmt::Display for LensOnly<L>
where
    L: Readable + 'static,
    L::Target: std::fmt::Display + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| std::fmt::Display::fmt(v, f))
    }
}

impl<L> std::fmt::Debug for LensOnly<L>
where
    L: Readable + 'static,
    L::Target: std::fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| std::fmt::Debug::fmt(v, f))
    }
}

impl<L> PartialEq for LensOnly<L>
where
    L: Readable + PartialEq + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.lens == other.lens
    }
}

impl<L> std::ops::Deref for LensOnly<L>
where
    L: Readable + 'static,
    L::Target: Clone + 'static + Sized,
{
    type Target = dyn Fn() -> L::Target;
    fn deref(&self) -> &Self::Target {
        // Safety: same pattern as Store's Deref impl.
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<L> ProjectMap for LensOnly<L>
where
    L: Readable,
    L::Target: 'static,
{
    type Lens = L;

    type Rebind<U: ?Sized + 'static, Lens>
        = LensOnly<Lens>
    where
        Lens: Readable<Target = U, Storage = L::Storage> + 'static;

    fn project_compose<U, Lens>(self, map: impl FnOnce(Self::Lens) -> Lens) -> Self::Rebind<U, Lens>
    where
        U: ?Sized + 'static,
        Lens: Readable<Target = U, Storage = L::Storage> + 'static,
    {
        LensOnly {
            lens: map(self.lens),
        }
    }

    fn project_lens(&self) -> &L {
        &self.lens
    }

    fn project_write_untracked(&self) -> WritableRef<'static, L>
    where
        L: Writable,
        L::Target: 'static,
    {
        // For LensOnly, this WILL notify subscribers via the signal's normal
        // write path (there's no separate "tracked" write). That's fine: the
        // whole point of LensOnly is to forego the path-level granularity and
        // just use the signal's own subscription list.
        self.lens.write_unchecked()
    }
}

impl<L> ProjectScope for LensOnly<L>
where
    L: Readable,
    L::Target: 'static,
{
    fn project_key(self, _key: PathKey) -> Self {
        self
    }

    fn project_hash_key<K: Hash + ?Sized>(self, _key: &K) -> Self {
        self
    }

    fn project_track_shallow(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.lens.subscribers());
        }
    }
    fn project_track(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.lens.subscribers());
        }
    }
    fn project_mark_dirty(&self) {
        // LensOnly has no path tree — a signal write notifies all subscribers,
        // so we have nothing extra to do here. The caller pairs this with
        // `project_write_untracked` which itself only bypasses tracking; when
        // combined, the caller is responsible for using a normal write to
        // notify subscribers (see the LensOnly convention below).
    }
    fn project_mark_dirty_shallow(&self) {}
    fn project_mark_dirty_at_and_after_index(&self, _index: usize) {}
    fn project_mark_hash_child_dirty<K: std::hash::Hash + ?Sized>(&self, _key: &K) {}
}

// ---------------------------------------------------------------------------
// Raw signal carriers — whole-signal tracking, pathless child scoping.
// ---------------------------------------------------------------------------

impl<T, S> ProjectMap for Signal<T, S>
where
    S: AnyStorage,
    Signal<T, S>: Writable<Target = T, Storage = S> + 'static,
    T: 'static,
{
    type Lens = Self;

    type Rebind<U: ?Sized + 'static, L>
        = LensOnly<L>
    where
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

    fn project_compose<U, L>(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>
    where
        U: ?Sized + 'static,
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static,
    {
        LensOnly::new(map(self))
    }

    fn project_lens(&self) -> &Self::Lens {
        self
    }

    fn project_write_untracked(&self) -> WritableRef<'static, Self::Lens>
    where
        Self::Lens: Writable,
        <Self::Lens as Readable>::Target: 'static,
    {
        self.write_unchecked()
    }
}

impl<T, S> ProjectScope for Signal<T, S>
where
    S: AnyStorage,
    Signal<T, S>: Writable<Target = T, Storage = S> + 'static,
    T: 'static,
{
    fn project_key(self, _key: PathKey) -> Self {
        self
    }

    fn project_hash_key<K: Hash + ?Sized>(self, _key: &K) -> Self {
        self
    }

    fn project_track_shallow(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers());
        }
    }

    fn project_track(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers());
        }
    }

    fn project_mark_dirty(&self) {}

    fn project_mark_dirty_shallow(&self) {}

    fn project_mark_dirty_at_and_after_index(&self, _index: usize) {}

    fn project_mark_hash_child_dirty<K: Hash + ?Sized>(&self, _key: &K) {}
}

impl<T: ?Sized + 'static, S> ProjectMap for ReadSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    ReadSignal<T, S>: Readable<Target = T, Storage = S> + 'static,
{
    type Lens = Self;

    type Rebind<U: ?Sized + 'static, L>
        = LensOnly<L>
    where
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

    fn project_compose<U, L>(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>
    where
        U: ?Sized + 'static,
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static,
    {
        LensOnly::new(map(self))
    }

    fn project_lens(&self) -> &Self::Lens {
        self
    }

    fn project_write_untracked(&self) -> WritableRef<'static, Self::Lens>
    where
        Self::Lens: Writable,
        <Self::Lens as Readable>::Target: 'static,
    {
        unreachable!("ReadSignal projections are read-only")
    }
}

impl<T: ?Sized + 'static, S> ProjectScope for ReadSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    ReadSignal<T, S>: Readable<Target = T, Storage = S> + 'static,
{
    fn project_key(self, _key: PathKey) -> Self {
        self
    }

    fn project_hash_key<K: Hash + ?Sized>(self, _key: &K) -> Self {
        self
    }

    fn project_track_shallow(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers());
        }
    }

    fn project_track(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers());
        }
    }

    fn project_mark_dirty(&self) {}

    fn project_mark_dirty_shallow(&self) {}

    fn project_mark_dirty_at_and_after_index(&self, _index: usize) {}

    fn project_mark_hash_child_dirty<K: Hash + ?Sized>(&self, _key: &K) {}
}

impl<T: ?Sized + 'static, S> ProjectMap for WriteSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    WriteSignal<T, S>: Writable<Target = T, Storage = S> + 'static,
{
    type Lens = Self;

    type Rebind<U: ?Sized + 'static, L>
        = LensOnly<L>
    where
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

    fn project_compose<U, L>(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>
    where
        U: ?Sized + 'static,
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static,
    {
        LensOnly::new(map(self))
    }

    fn project_lens(&self) -> &Self::Lens {
        self
    }

    fn project_write_untracked(&self) -> WritableRef<'static, Self::Lens>
    where
        Self::Lens: Writable,
        <Self::Lens as Readable>::Target: 'static,
    {
        self.write_unchecked()
    }
}

impl<T: ?Sized + 'static, S> ProjectScope for WriteSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    WriteSignal<T, S>: Writable<Target = T, Storage = S> + 'static,
{
    fn project_key(self, _key: PathKey) -> Self {
        self
    }

    fn project_hash_key<K: Hash + ?Sized>(self, _key: &K) -> Self {
        self
    }

    fn project_track_shallow(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers());
        }
    }

    fn project_track(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers());
        }
    }

    fn project_mark_dirty(&self) {}

    fn project_mark_dirty_shallow(&self) {}

    fn project_mark_dirty_at_and_after_index(&self, _index: usize) {}

    fn project_mark_hash_child_dirty<K: Hash + ?Sized>(&self, _key: &K) {}
}

// ---------------------------------------------------------------------------
// Shape: indexing
// ---------------------------------------------------------------------------

/// Projection through `IndexMut` to a child at `index`.
pub trait ProjectIndex<Idx>:
    ProjectScope<Lens: Readable<Target: IndexMut<Idx> + IndexSelector<Idx>>>
{
    /// Project into the item at `index`.
    fn index(
        self,
        index: Idx,
    ) -> Self::Rebind<
        <<Self::Lens as Readable>::Target as Index<Idx>>::Output,
        IndexWrite<Idx, Self::Lens>,
    >
    where
        Idx: Clone + 'static,
        Self::Lens: 'static,
        <Self::Lens as Readable>::Target: 'static,
    {
        <<Self::Lens as Readable>::Target as IndexSelector<Idx>>::scope_project(self, &index)
            .project_compose(|lens| IndexWrite::new(index, lens))
    }
}

impl<Idx, P> ProjectIndex<Idx> for P where
    P: ProjectScope<Lens: Readable<Target: IndexMut<Idx> + IndexSelector<Idx>>>
{
}

// ---------------------------------------------------------------------------
// Shape: HashMap<K, V, St>
// ---------------------------------------------------------------------------

/// Read-side methods on `HashMap<K, V, St>` projections.
pub trait ProjectHashMap<K: 'static, V: 'static, St: 'static>:
    ProjectScope<Lens: Readable<Target = HashMap<K, V, St>>>
{
    /// Map length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_peek().len()
    }

    /// Is the map empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_empty()
    }

    /// Check whether a key exists; tracks shallowly.
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.project_track_shallow();
        self.project_peek().contains_key(key)
    }

    /// Iterate entries as `(key, value-projection)` pairs.
    fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (K, Self::Rebind<V, HashMapGetWrite<K, Self::Lens>>)>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Eq + Hash + Clone,
        St: BuildHasher,
        Self: Clone,
        Self::Lens: 'static,
    {
        ProjectHashMap::<K, V, St>::len(self);
        let keys: Vec<_> = self.project_peek().keys().cloned().collect();
        let this = self.clone();
        keys.into_iter()
            .map(move |key| (key.clone(), this.clone().get_unchecked(key)))
    }

    /// Iterate values as projections.
    fn values(
        &self,
    ) -> impl ExactSizeIterator<Item = Self::Rebind<V, HashMapGetWrite<K, Self::Lens>>>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Eq + Hash + Clone,
        St: BuildHasher,
        Self: Clone,
        Self::Lens: 'static,
    {
        ProjectHashMap::<K, V, St>::len(self);
        let keys = self.project_peek().keys().cloned().collect::<Vec<_>>();
        let this = self.clone();
        keys.into_iter()
            .map(move |key| this.clone().get_unchecked(key))
    }

    /// Get a projection for the value associated with `key`.
    fn get<Q>(self, key: Q) -> Option<Self::Rebind<V, HashMapGetWrite<Q, Self::Lens>>>
    where
        Q: Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
        Self::Lens: 'static,
    {
        self.contains_key(&key).then(|| self.get_unchecked(key))
    }

    /// Get a projection for the value at `key` without existence check.
    #[track_caller]
    fn get_unchecked<Q>(self, key: Q) -> Self::Rebind<V, HashMapGetWrite<Q, Self::Lens>>
    where
        Q: Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
        Self::Lens: 'static,
    {
        let created = Location::caller();
        self.project_hash_key(key.borrow())
            .project_compose(|lens| HashMapGetWrite::new(key, lens, created))
    }
}

impl<K, V, St, P> ProjectHashMap<K, V, St> for P
where
    K: 'static,
    V: 'static,
    St: 'static,
    P: ProjectScope<Lens: Readable<Target = HashMap<K, V, St>>>,
{
}

/// Mutation methods on `HashMap<K, V, St>` projections.
pub trait ProjectHashMapMut<K: 'static, V: 'static, St: 'static>:
    ProjectScope<Lens: Writable<Target = HashMap<K, V, St>>>
{
    /// Insert a key-value pair; marks shape dirty + existing child at the hash path dirty.
    fn insert(&self, key: K, value: V)
    where
        K: Eq + Hash,
        St: BuildHasher,
    {
        self.project_mark_dirty_shallow();
        self.project_mark_hash_child_dirty(&key);
        self.project_write_untracked().insert(key, value);
    }

    /// Remove a key; marks shape dirty.
    fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().remove(key)
    }

    /// Clear the map; marks shape dirty.
    fn clear(&self) {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().clear();
    }

    /// Retain only entries matching `f`; marks shape dirty.
    fn retain(&self, mut f: impl FnMut(&K, &V) -> bool) {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().retain(|k, v| f(k, v));
    }
}

impl<K, V, St, P> ProjectHashMapMut<K, V, St> for P
where
    K: 'static,
    V: 'static,
    St: 'static,
    P: ProjectScope<Lens: Writable<Target = HashMap<K, V, St>>>,
{
}

// ---------------------------------------------------------------------------
// Shape: BTreeMap<K, V>
// ---------------------------------------------------------------------------

/// Read-side methods on `BTreeMap<K, V>` projections.
pub trait ProjectBTreeMap<K: 'static, V: 'static>:
    ProjectScope<Lens: Readable<Target = BTreeMap<K, V>>>
{
    /// Map length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_peek().len()
    }

    /// Is the map empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_empty()
    }

    /// Check whether a key exists; tracks shallowly.
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Ord,
        K: Borrow<Q> + Ord,
    {
        self.project_track_shallow();
        self.project_peek().contains_key(key)
    }

    /// Iterate the map, producing one value projection per key.
    fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (K, Self::Rebind<V, BTreeMapGetWrite<K, Self::Lens>>)>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Hash + Ord + Clone,
        Self: Clone,
        Self::Lens: 'static,
    {
        ProjectBTreeMap::<K, V>::len(self);
        let keys: Vec<_> = self.project_peek().keys().cloned().collect();
        let this = self.clone();
        keys.into_iter().map(move |key| {
            let value = this.clone().get_unchecked(key.clone());
            (key, value)
        })
    }

    /// Iterate the map values as projections.
    fn values(
        &self,
    ) -> impl ExactSizeIterator<Item = Self::Rebind<V, BTreeMapGetWrite<K, Self::Lens>>>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        K: Hash + Ord + Clone,
        Self: Clone,
        Self::Lens: 'static,
    {
        ProjectBTreeMap::<K, V>::len(self);
        let keys = self.project_peek().keys().cloned().collect::<Vec<_>>();
        let this = self.clone();
        keys.into_iter()
            .map(move |key| this.clone().get_unchecked(key))
    }

    /// Get a projection for the value at `key` if it exists.
    fn get<Q>(self, key: Q) -> Option<Self::Rebind<V, BTreeMapGetWrite<Q, Self::Lens>>>
    where
        Q: Hash + Ord + 'static,
        K: Borrow<Q> + Ord,
        Self::Lens: 'static,
    {
        self.contains_key(&key).then(|| self.get_unchecked(key))
    }

    /// Get a projection for the value at `key` without checking existence.
    #[track_caller]
    fn get_unchecked<Q>(self, key: Q) -> Self::Rebind<V, BTreeMapGetWrite<Q, Self::Lens>>
    where
        Q: Hash + Ord + 'static,
        K: Borrow<Q> + Ord,
        Self::Lens: 'static,
    {
        let created = Location::caller();
        self.project_hash_key(key.borrow())
            .project_compose(|lens| BTreeMapGetWrite::new(key, lens, created))
    }
}

impl<K, V, P> ProjectBTreeMap<K, V> for P
where
    K: 'static,
    V: 'static,
    P: ProjectScope<Lens: Readable<Target = BTreeMap<K, V>>>,
{
}

/// Mutation methods on `BTreeMap<K, V>` projections.
pub trait ProjectBTreeMapMut<K: 'static, V: 'static>:
    ProjectScope<Lens: Writable<Target = BTreeMap<K, V>>>
{
    /// Insert a key-value pair; marks shape dirty.
    fn insert(&self, key: K, value: V) -> Option<V>
    where
        K: Ord + Hash,
    {
        self.project_mark_dirty_shallow();
        self.project_mark_hash_child_dirty(&key);
        self.project_write_untracked().insert(key, value)
    }

    /// Remove a key; marks shape dirty.
    fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Ord,
        K: Borrow<Q> + Ord,
    {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().remove(key)
    }

    /// Clear the map; marks shape dirty.
    fn clear(&self) {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().clear();
    }

    /// Retain only entries matching `f`; marks shape dirty.
    fn retain(&self, mut f: impl FnMut(&K, &mut V) -> bool)
    where
        K: Ord,
    {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().retain(|k, v| f(k, v));
    }
}

impl<K, V, P> ProjectBTreeMapMut<K, V> for P
where
    K: 'static,
    V: 'static,
    P: ProjectScope<Lens: Writable<Target = BTreeMap<K, V>>>,
{
}
