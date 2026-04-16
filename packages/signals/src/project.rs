//! Generic projection primitives built on top of signals.
//!
//! [`ProjectLens`] is the functional carrier/lens mapping layer.
//! [`ProjectPath`] adds keyed path composition.
//! [`ProjectReact`] adds subscription tracking and dirty marking on top.
//! [`Project`] is the convenience umbrella for projector carriers that do the
//! lens/path/reactive pieces. It's implemented directly for pathless signal
//! carriers in this crate, and store-backed carriers add their implementations
//! in `dioxus-stores`.

use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::hash::{BuildHasher, Hash, Hasher};
use std::iter::FusedIterator;
use std::ops::{DerefMut, Index, IndexMut};
use std::panic::Location;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{
    AnyStorage, BorrowError, BorrowMutError, BoxedSignalStorage, CreateBoxedSignalStorage,
    MappedMutSignal, ReadSignal, Readable, ReadableExt, ReadableRef, Signal, SyncStorage,
    UnsyncStorage, Writable, WritableExt, WritableRef, WriteLock, WriteSignal,
};
use dioxus_core::{ReactiveContext, Subscribers};
use generational_box::ValueDroppedError;

/// The path segment type used by projector carriers.
#[doc(hidden)]
pub type PathKey = u16;

/// Hash an index into a `PathKey` using the deterministic default SipHasher.
#[doc(hidden)]
pub fn hash_path_key(index: &(impl Hash + ?Sized)) -> PathKey {
    let mut hasher = DefaultHasher::new();
    index.hash(&mut hasher);
    (hasher.finish() % PathKey::MAX as u64) as PathKey
}

/// The lens produced by mapping a projector through `F` / `FMut`.
#[doc(hidden)]
#[allow(type_alias_bounds)]
pub type MappedProjectLens<
    P: ProjectLens,
    U: ?Sized,
    F = fn(&<<P as ProjectLens>::Lens as Readable>::Target) -> &U,
    FMut = fn(&mut <<P as ProjectLens>::Lens as Readable>::Target) -> &mut U,
> where
    P::Lens: Readable,
= MappedMutSignal<U, <P as ProjectLens>::Lens, F, FMut>;

/// Rebind a projector carrier to a mapped child lens.
#[allow(type_alias_bounds)]
pub type Projected<
    P: ProjectLens,
    U: ?Sized,
    F = fn(&<<P as ProjectLens>::Lens as Readable>::Target) -> &U,
    FMut = fn(&mut <<P as ProjectLens>::Lens as Readable>::Target) -> &mut U,
> where
    P::Lens: Readable,
= <P as ProjectLens>::Rebind<U, MappedProjectLens<P, U, F, FMut>>;

/// Abstracts over "same projection semantics, different lens in the same storage".
pub trait ProjectLens: Sized {
    /// The lens this projection reads/writes through.
    type Lens;

    /// Rebind this carrier to a different lens/target pair while preserving
    /// its projection semantics.
    type Rebind<U: ?Sized + 'static, L>
    where
        Self::Lens: Readable,
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

    /// Borrow the underlying lens.
    fn project_lens(&self) -> &Self::Lens;

    /// Rebuild this carrier around a derived lens built from the current one.
    fn project_compose<U, L>(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>
    where
        Self::Lens: Readable,
        U: ?Sized + 'static,
        L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static,
        Self: ProjectCompose<U, L>,
    {
        <Self as ProjectCompose<U, L>>::project_compose_inner(self, map)
    }
}

/// Carrier-specific composition rules for a projected child lens.
pub trait ProjectCompose<U: ?Sized + 'static, L>: ProjectLens
where
    Self::Lens: Readable,
    L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static,
{
    /// Rebuild this carrier around a derived lens built from the current one.
    fn project_compose_inner(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L>;
}

/// Convenience methods built on top of [`ProjectLens`].
pub trait ProjectLensExt: ProjectLens {
    /// Map the lens without introducing a path-level child.
    fn project_map<U, F, FMut>(self, map: F, map_mut: FMut) -> Projected<Self, U, F, FMut>
    where
        Self::Lens: Readable,
        U: ?Sized + 'static,
        <Self::Lens as Readable>::Target: 'static,
        F: Fn(&<Self::Lens as Readable>::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut <Self::Lens as Readable>::Target) -> &mut U + 'static + Copy,
        Self: ProjectCompose<U, MappedProjectLens<Self, U, F, FMut>>,
    {
        self.project_compose::<U, _>(|lens| MappedMutSignal::new(lens, map, map_mut))
    }
}

impl<T> ProjectLensExt for T where T: ProjectLens {}

/// Keyed path composition for a projector carrier.
pub trait ProjectPath: Sized {
    /// Scope this projection to a keyed child without changing the lens type.
    fn project_key(self, key: PathKey) -> Self;
}

/// Convenience methods built on top of [`ProjectPath`].
pub trait ProjectPathExt: ProjectPath + ProjectLens {
    /// Scope into a keyed child, wrapping the lens with a new map/map_mut.
    fn project_child<U, F, FMut>(
        self,
        key: PathKey,
        map: F,
        map_mut: FMut,
    ) -> Projected<Self, U, F, FMut>
    where
        Self::Lens: Readable,
        U: ?Sized + 'static,
        <Self::Lens as Readable>::Target: 'static,
        F: Fn(&<Self::Lens as Readable>::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut <Self::Lens as Readable>::Target) -> &mut U + 'static + Copy,
        Self: ProjectCompose<U, MappedProjectLens<Self, U, F, FMut>>,
    {
        self.project_key(key).project_map(map, map_mut)
    }
}

impl<T> ProjectPathExt for T where T: ProjectPath + ProjectLens {}

/// Convenience methods for hashing arbitrary keys into the projector path space.
pub trait ProjectHashExt: ProjectPath {
    /// Scope this projection to a hashed child without changing the lens type.
    fn project_hash_key<K: Hash + ?Sized>(self, key: &K) -> Self {
        self.project_key(hash_path_key(key))
    }
}

impl<T> ProjectHashExt for T where T: ProjectPath {}

/// Subscription tracking and dirty marking for a projector carrier.
pub trait ProjectReact {
    /// Track only exact writes at this node.
    fn project_track_shallow(&self);

    /// Track writes at this node and any descendant.
    fn project_track(&self);

    /// Mark this node and all descendants dirty.
    fn project_mark_dirty(&self);

    /// Mark just this node dirty.
    fn project_mark_dirty_shallow(&self);

    /// Mark indexed descendants at and after the provided index dirty.
    fn project_mark_dirty_at_and_after_index(&self, index: usize);
}

/// Convenience umbrella for carriers that support both lens mapping and
/// reactive path scoping.
pub trait Project: ProjectLens + ProjectPath + ProjectReact {}

impl<T> Project for T where T: ProjectLens + ProjectPath + ProjectReact {}

macro_rules! impl_raw_project_lens {
    ($storage:ty) => {
        impl<T> ProjectLens for Signal<T, $storage>
        where
            Signal<T, $storage>: Writable<Target = T, Storage = $storage> + 'static,
            T: 'static,
        {
            type Lens = Self;

            type Rebind<U: ?Sized + 'static, L>
                = WriteSignal<U, $storage>
            where
                L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

            fn project_lens(&self) -> &Self::Lens {
                self
            }
        }

        impl<T, U, L> ProjectCompose<U, L> for Signal<T, $storage>
        where
            Signal<T, $storage>: Writable<Target = T, Storage = $storage> + 'static,
            T: 'static,
            U: ?Sized + 'static,
            L: Writable<Target = U, Storage = $storage, WriteMetadata: 'static> + 'static,
            $storage: CreateBoxedSignalStorage<L>,
        {
            fn project_compose_inner(
                self,
                map: impl FnOnce(Self::Lens) -> L,
            ) -> Self::Rebind<U, L> {
                WriteSignal::new_maybe_sync(map(self))
            }
        }

        impl<T: ?Sized + 'static> ProjectLens for ReadSignal<T, $storage>
        where
            ReadSignal<T, $storage>: Readable<Target = T, Storage = $storage> + 'static,
        {
            type Lens = Self;

            type Rebind<U: ?Sized + 'static, L>
                = ReadSignal<U, $storage>
            where
                L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

            fn project_lens(&self) -> &Self::Lens {
                self
            }
        }

        impl<T: ?Sized + 'static, U, L> ProjectCompose<U, L> for ReadSignal<T, $storage>
        where
            ReadSignal<T, $storage>: Readable<Target = T, Storage = $storage> + 'static,
            U: ?Sized + 'static,
            L: Readable<Target = U, Storage = $storage> + 'static,
            $storage: CreateBoxedSignalStorage<L>,
        {
            fn project_compose_inner(
                self,
                map: impl FnOnce(Self::Lens) -> L,
            ) -> Self::Rebind<U, L> {
                ReadSignal::new_maybe_sync(map(self))
            }
        }

        impl<T: ?Sized + 'static> ProjectLens for WriteSignal<T, $storage>
        where
            WriteSignal<T, $storage>: Writable<Target = T, Storage = $storage> + 'static,
        {
            type Lens = Self;

            type Rebind<U: ?Sized + 'static, L>
                = WriteSignal<U, $storage>
            where
                L: Readable<Target = U, Storage = <Self::Lens as Readable>::Storage> + 'static;

            fn project_lens(&self) -> &Self::Lens {
                self
            }
        }

        impl<T: ?Sized + 'static, U, L> ProjectCompose<U, L> for WriteSignal<T, $storage>
        where
            WriteSignal<T, $storage>: Writable<Target = T, Storage = $storage> + 'static,
            U: ?Sized + 'static,
            L: Writable<Target = U, Storage = $storage, WriteMetadata: 'static> + 'static,
            $storage: CreateBoxedSignalStorage<L>,
        {
            fn project_compose_inner(
                self,
                map: impl FnOnce(Self::Lens) -> L,
            ) -> Self::Rebind<U, L> {
                WriteSignal::new_maybe_sync(map(self))
            }
        }
    };
}

impl_raw_project_lens!(UnsyncStorage);
impl_raw_project_lens!(SyncStorage);

impl<T, S> ProjectPath for Signal<T, S>
where
    S: AnyStorage,
    Signal<T, S>: Writable<Target = T, Storage = S> + 'static,
    T: 'static,
{
    fn project_key(self, _key: PathKey) -> Self {
        self
    }
}

impl<T, S> ProjectReact for Signal<T, S>
where
    S: AnyStorage,
    Signal<T, S>: Writable<Target = T, Storage = S> + 'static,
    T: 'static,
{
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
}

impl<T: ?Sized + 'static, S> ProjectPath for ReadSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    ReadSignal<T, S>: Readable<Target = T, Storage = S> + 'static,
{
    fn project_key(self, _key: PathKey) -> Self {
        self
    }
}

impl<T: ?Sized + 'static, S> ProjectReact for ReadSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    ReadSignal<T, S>: Readable<Target = T, Storage = S> + 'static,
{
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
}

impl<T: ?Sized + 'static, S> ProjectPath for WriteSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    WriteSignal<T, S>: Writable<Target = T, Storage = S> + 'static,
{
    fn project_key(self, _key: PathKey) -> Self {
        self
    }
}

impl<T: ?Sized + 'static, S> ProjectReact for WriteSignal<T, S>
where
    S: AnyStorage + BoxedSignalStorage<T>,
    WriteSignal<T, S>: Writable<Target = T, Storage = S> + 'static,
{
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
}

mod sealed {
    pub trait IndexSelector<Idx, P> {}
}

/// The way a data structure scopes a projector to one of its indexed children.
#[doc(hidden)]
pub trait IndexSelector<Idx, P>: sealed::IndexSelector<Idx, P> {
    /// Given a projection and an index, scope it to the child at that index.
    fn scope_project(project: P, index: &Idx) -> P;
}

impl<T, P> IndexSelector<usize, P> for Vec<T>
where
    P: ProjectPath,
{
    fn scope_project(project: P, index: &usize) -> P {
        project.project_key(*index as _)
    }
}

impl<T, P> sealed::IndexSelector<usize, P> for Vec<T> where P: ProjectPath {}

impl<T, P> IndexSelector<usize, P> for [T]
where
    P: ProjectPath,
{
    fn scope_project(project: P, index: &usize) -> P {
        project.project_key(*index as _)
    }
}

impl<T, P> sealed::IndexSelector<usize, P> for [T] where P: ProjectPath {}

impl<K, V, I, P> IndexSelector<I, P> for HashMap<K, V>
where
    I: Hash,
    P: ProjectPath,
{
    fn scope_project(project: P, index: &I) -> P {
        project.project_hash_key(index)
    }
}

impl<K, V, I, P> sealed::IndexSelector<I, P> for HashMap<K, V>
where
    I: Hash,
    P: ProjectPath,
{
}

impl<K, V, I, P> IndexSelector<I, P> for BTreeMap<K, V>
where
    I: Hash,
    P: ProjectPath,
{
    fn scope_project(project: P, index: &I) -> P {
        project.project_hash_key(index)
    }
}

impl<K, V, I, P> sealed::IndexSelector<I, P> for BTreeMap<K, V>
where
    I: Hash,
    P: ProjectPath,
{
}

/// A specific index in a `Readable` / `Writable` type.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct IndexWrite<Index, Write> {
    index: Index,
    write: Write,
}

impl<Index, Write> IndexWrite<Index, Write> {
    pub(crate) fn new(index: Index, write: Write) -> Self {
        Self { index, write }
    }
}

impl<Index, Write> Readable for IndexWrite<Index, Write>
where
    Write: Readable,
    Write::Target: std::ops::Index<Index> + 'static,
    Index: Clone,
{
    type Target = <Write::Target as std::ops::Index<Index>>::Output;
    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().map(|value| {
            Self::Storage::map(value, |value: &Write::Target| {
                value.index(self.index.clone())
            })
        })
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().map(|value| {
            Self::Storage::map(value, |value: &Write::Target| {
                value.index(self.index.clone())
            })
        })
    }

    fn subscribers(&self) -> Subscribers
    where
        Self::Target: 'static,
    {
        self.write.subscribers()
    }
}

impl<Index, Write> Writable for IndexWrite<Index, Write>
where
    Write: Writable,
    Write::Target: std::ops::IndexMut<Index> + 'static,
    Index: Clone,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError>
    where
        Self::Target: 'static,
    {
        self.write.try_write_unchecked().map(|value| {
            WriteLock::map(value, |value: &mut Write::Target| {
                value.index_mut(self.index.clone())
            })
        })
    }
}

/// A specific key in a `Readable` / `Writable` `HashMap`.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct HashMapGetWrite<Index, Write> {
    index: Index,
    write: Write,
    created: &'static Location<'static>,
}

impl<Index, Write> HashMapGetWrite<Index, Write> {
    pub(crate) fn new(index: Index, write: Write, created: &'static Location<'static>) -> Self {
        Self {
            index,
            write,
            created,
        }
    }
}

impl<Index, Write, K, V, St> Readable for HashMapGetWrite<Index, Write>
where
    Write: Readable<Target = HashMap<K, V, St>>,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
{
    type Target = V;
    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn subscribers(&self) -> Subscribers
    where
        Self::Target: 'static,
    {
        self.write.subscribers()
    }
}

impl<Index, Write, K, V, St> Writable for HashMapGetWrite<Index, Write>
where
    Write: Writable<Target = HashMap<K, V, St>>,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError>
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

/// A specific key in a `Readable` / `Writable` `BTreeMap`.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct BTreeMapGetWrite<Index, Write> {
    index: Index,
    write: Write,
    created: &'static Location<'static>,
}

impl<Index, Write> BTreeMapGetWrite<Index, Write> {
    pub(crate) fn new(index: Index, write: Write, created: &'static Location<'static>) -> Self {
        Self {
            index,
            write,
            created,
        }
    }
}

impl<Index, Write, K, V> Readable for BTreeMapGetWrite<Index, Write>
where
    Write: Readable<Target = BTreeMap<K, V>>,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
{
    type Target = V;
    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn subscribers(&self) -> Subscribers
    where
        Self::Target: 'static,
    {
        self.write.subscribers()
    }
}

impl<Index, Write, K, V> Writable for BTreeMapGetWrite<Index, Write>
where
    Write: Writable<Target = BTreeMap<K, V>>,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError>
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

/// Project through a `DerefMut` target without introducing a new path subscription.
pub trait ProjectDeref<U: ?Sized + 'static>:
    ProjectLens<Lens: Readable<Target: DerefMut<Target = U>>>
{
    /// Project through `DerefMut` to the inner target.
    fn deref(self) -> Projected<Self, U>
    where
        <Self::Lens as Readable>::Target: 'static,
        Self: ProjectCompose<U, MappedProjectLens<Self, U>>,
    {
        let map: fn(&<Self::Lens as Readable>::Target) -> &U = |t| &**t;
        let map_mut: fn(&mut <Self::Lens as Readable>::Target) -> &mut U = |t| &mut **t;
        self.project_map(map, map_mut)
    }
}

impl<U: ?Sized + 'static, P> ProjectDeref<U> for P where
    P: ProjectLens<Lens: Readable<Target: DerefMut<Target = U>>>
{
}

/// Projection methods for types targeting `Option<T>`.
pub trait ProjectOption<T: 'static>: Project<Lens: Readable<Target = Option<T>>> {
    /// Is the option currently `Some`? Tracks shallowly.
    fn is_some(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_some()
    }

    /// Is the option currently `None`? Tracks shallowly.
    fn is_none(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_none()
    }

    /// Tracks shallowly and deeply if the option is `Some`.
    fn is_some_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        if let Some(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v)
        } else {
            false
        }
    }

    /// Tracks shallowly and deeply if the option is `Some`.
    fn is_none_or(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        if let Some(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v)
        } else {
            true
        }
    }

    /// Transpose `Self<Option<T>>` into `Option<Self<T>>`.
    fn transpose(self) -> Option<Projected<Self, T>>
    where
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>,
    {
        if self.is_some() {
            let map: fn(&Option<T>) -> &T = |v| {
                v.as_ref()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            let map_mut: fn(&mut Option<T>) -> &mut T = |v| {
                v.as_mut()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Unwrap to `Self<T>`; panics if currently `None`.
    fn unwrap(self) -> Projected<Self, T>
    where
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>,
    {
        self.transpose()
            .unwrap_or_else(|| panic!("called `unwrap` on a `None` Option projection"))
    }

    /// Unwrap to `Self<T>`; panics with `msg` if currently `None`.
    fn expect(self, msg: &'static str) -> Projected<Self, T>
    where
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>,
    {
        self.transpose().unwrap_or_else(|| panic!("{}", msg))
    }

    /// Return a `[T]` view of the option: `&[value]` if `Some`, `&[]` if `None`.
    fn as_slice(self) -> Projected<Self, [T]>
    where
        T: Sized,
        Self: ProjectCompose<[T], MappedProjectLens<Self, [T]>>,
    {
        let map: fn(&Option<T>) -> &[T] = Option::as_slice;
        let map_mut: fn(&mut Option<T>) -> &mut [T] = Option::as_mut_slice;
        self.project_map(map, map_mut)
    }

    /// Project through `Deref` on the contained value.
    fn as_deref(self) -> Option<Projected<Self, T::Target>>
    where
        T: DerefMut,
        T::Target: 'static,
        Self: ProjectCompose<T::Target, MappedProjectLens<Self, T::Target>>,
    {
        if self.is_some() {
            let map: fn(&Option<T>) -> &T::Target = |v| {
                (&**v
                    .as_ref()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value")))
                    as &T::Target
            };
            let map_mut: fn(&mut Option<T>) -> &mut T::Target = |v| {
                &mut **v
                    .as_mut()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Filter the option by a predicate. Always tracks shallowly; tracks deeply when `Some`.
    fn filter(self, f: impl FnOnce(&T) -> bool) -> Option<Projected<Self, T>>
    where
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>,
    {
        if self.is_some_and(f) {
            let map: fn(&Option<T>) -> &T = |v| {
                v.as_ref()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            let map_mut: fn(&mut Option<T>) -> &mut T = |v| {
                v.as_mut()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Peek at the inner value if `Some`; tracks shallowly, and deeply when `Some`.
    fn inspect(self, f: impl FnOnce(&T)) -> Self {
        self.project_track_shallow();
        if let Some(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v);
        }
        self
    }
}

impl<T: 'static, P> ProjectOption<T> for P where P: Project<Lens: Readable<Target = Option<T>>> {}

/// Projection methods for types targeting `Result<T, E>`.
pub trait ProjectResult<T: 'static, E: 'static>:
    Project<Lens: Readable<Target = Result<T, E>>>
{
    /// Returns `true` if the projected result is `Ok`.
    fn is_ok(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_ok()
    }

    /// Returns `true` if the projected result is `Err`.
    fn is_err(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_err()
    }

    /// Returns `true` if the projected result is `Ok` and the predicate matches the inner value.
    fn is_ok_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        match &*self.project_lens().peek_unchecked() {
            Ok(v) => {
                self.project_track();
                f(v)
            }
            Err(_) => false,
        }
    }

    /// Returns `true` if the projected result is `Err` and the predicate matches the inner error.
    fn is_err_and(&self, f: impl FnOnce(&E) -> bool) -> bool {
        self.project_track_shallow();
        match &*self.project_lens().peek_unchecked() {
            Err(e) => {
                self.project_track();
                f(e)
            }
            Ok(_) => false,
        }
    }

    /// Projects the `Ok` variant when present.
    fn ok(self) -> Option<Projected<Self, T>>
    where
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>,
    {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T = |r| {
                r.as_ref()
                    .unwrap_or_else(|_| panic!("Tried to access `Ok` on a `Result` value"))
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut T = |r| {
                r.as_mut()
                    .unwrap_or_else(|_| panic!("Tried to access `Ok` on a `Result` value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Projects the `Err` variant when present.
    fn err(self) -> Option<Projected<Self, E>>
    where
        Self: ProjectCompose<E, MappedProjectLens<Self, E>>,
    {
        if self.is_err() {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on a `Result` value")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on a `Result` value")
                }
            };
            Some(self.project_child(1, map, map_mut))
        } else {
            None
        }
    }

    /// Transposes `Self<Result<T, E>>` into `Result<Self<T>, Self<E>>`.
    #[allow(clippy::type_complexity)]
    fn transpose(self) -> Result<Projected<Self, T>, Projected<Self, E>>
    where
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>
            + ProjectCompose<E, MappedProjectLens<Self, E>>,
    {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T =
                |r| r.as_ref().unwrap_or_else(|_| panic!("unreachable"));
            let map_mut: fn(&mut Result<T, E>) -> &mut T =
                |r| r.as_mut().unwrap_or_else(|_| panic!("unreachable"));
            Ok(self.project_child(0, map, map_mut))
        } else {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("unreachable")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("unreachable")
                }
            };
            Err(self.project_child(1, map, map_mut))
        }
    }

    /// Unwrap into `Self<T>`; panics if currently `Err`.
    fn unwrap(self) -> Projected<Self, T>
    where
        E: std::fmt::Debug,
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>
            + ProjectCompose<E, MappedProjectLens<Self, E>>,
    {
        match self.transpose() {
            Ok(ok) => ok,
            Err(_) => panic!("called `unwrap` on an Err Result projection"),
        }
    }

    /// Unwrap into `Self<T>`; panics with `msg` if currently `Err`.
    fn expect(self, msg: &'static str) -> Projected<Self, T>
    where
        E: std::fmt::Debug,
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>
            + ProjectCompose<E, MappedProjectLens<Self, E>>,
    {
        match self.transpose() {
            Ok(ok) => ok,
            Err(_) => panic!("{}", msg),
        }
    }

    /// Unwrap into `Self<E>`; panics if currently `Ok`.
    fn unwrap_err(self) -> Projected<Self, E>
    where
        T: std::fmt::Debug,
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>
            + ProjectCompose<E, MappedProjectLens<Self, E>>,
    {
        match self.transpose() {
            Err(e) => e,
            Ok(_) => panic!("called `unwrap_err` on an Ok Result projection"),
        }
    }

    /// Unwrap into `Self<E>`; panics with `msg` if currently `Ok`.
    fn expect_err(self, msg: &'static str) -> Projected<Self, E>
    where
        T: std::fmt::Debug,
        Self: ProjectCompose<T, MappedProjectLens<Self, T>>
            + ProjectCompose<E, MappedProjectLens<Self, E>>,
    {
        match self.transpose() {
            Err(e) => e,
            Ok(_) => panic!("{}", msg),
        }
    }

    /// Inspect the inner `Ok` value if present; tracks shallowly, and deeply when `Ok`.
    fn inspect(self, f: impl FnOnce(&T)) -> Self {
        self.project_track_shallow();
        if let Ok(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v);
        }
        self
    }

    /// Inspect the inner `Err` value if present; tracks shallowly, and deeply when `Err`.
    fn inspect_err(self, f: impl FnOnce(&E)) -> Self {
        self.project_track_shallow();
        if let Err(e) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(e);
        }
        self
    }

    /// Project through `Deref` on the `Ok` / `Err` variants.
    #[allow(clippy::type_complexity)]
    fn as_deref(self) -> Result<Projected<Self, T::Target>, Projected<Self, E>>
    where
        T: DerefMut,
        T::Target: 'static,
        Self: ProjectCompose<T::Target, MappedProjectLens<Self, T::Target>>
            + ProjectCompose<E, MappedProjectLens<Self, E>>,
    {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T::Target = |r| match r {
                Ok(t) => &**t,
                Err(_) => panic!("Tried to access `Ok` on an `Err` value"),
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut T::Target = |r| match r {
                Ok(t) => &mut **t,
                Err(_) => panic!("Tried to access `Ok` on an `Err` value"),
            };
            Ok(self.project_child(0, map, map_mut))
        } else {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on an `Ok` value")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on an `Ok` value")
                }
            };
            Err(self.project_child(1, map, map_mut))
        }
    }
}

impl<T: 'static, E: 'static, P> ProjectResult<T, E> for P where
    P: Project<Lens: Readable<Target = Result<T, E>>>
{
}

#[doc(hidden)]
pub trait ProjectIndexCompose<Idx>: ProjectLens + ProjectPath
where
    Idx: Clone + 'static,
    Self::Lens: Readable<Target: IndexMut<Idx> + IndexSelector<Idx, Self>>,
{
    fn project_index(
        self,
        index: Idx,
    ) -> Self::Rebind<
        <<Self::Lens as Readable>::Target as Index<Idx>>::Output,
        IndexWrite<Idx, Self::Lens>,
    >;
}

impl<Idx, P, Lens> ProjectIndexCompose<Idx> for P
where
    P: ProjectLens<Lens = Lens> + ProjectPath,
    Lens: Readable<Target: IndexMut<Idx> + IndexSelector<Idx, P>> + 'static,
    <Lens as Readable>::Target: 'static,
    Idx: Clone + 'static,
    P: ProjectCompose<<<Lens as Readable>::Target as Index<Idx>>::Output, IndexWrite<Idx, Lens>>,
{
    fn project_index(
        self,
        index: Idx,
    ) -> Self::Rebind<
        <<Self::Lens as Readable>::Target as Index<Idx>>::Output,
        IndexWrite<Idx, Self::Lens>,
    > {
        <<Self::Lens as Readable>::Target as IndexSelector<Idx, Self>>::scope_project(self, &index)
            .project_compose(|lens| IndexWrite::new(index, lens))
    }
}

/// Projection through `IndexMut` to a child at `index`.
pub trait ProjectIndex<Idx>: ProjectLens + ProjectPath
where
    Self::Lens: Readable<Target: IndexMut<Idx> + IndexSelector<Idx, Self>>,
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
        Self: ProjectIndexCompose<Idx>,
    {
        <Self as ProjectIndexCompose<Idx>>::project_index(self, index)
    }
}

impl<Idx, P> ProjectIndex<Idx> for P
where
    P: ProjectLens + ProjectPath,
    P::Lens: Readable<Target: IndexMut<Idx> + IndexSelector<Idx, P>>,
{
}

/// Read-side methods on `Vec<T>` projections.
pub trait ProjectSlice<T: 'static>: Project<Lens: Readable<Target = Vec<T>>> {
    /// Length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().len()
    }

    /// Is the slice empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_empty()
    }

    /// Iterate items, producing one indexed projection per element.
    fn iter(
        &self,
    ) -> impl ExactSizeIterator<
        Item = Self::Rebind<
            <<Self::Lens as Readable>::Target as Index<usize>>::Output,
            IndexWrite<usize, Self::Lens>,
        >,
    > + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        Self: Clone + ProjectIndex<usize> + ProjectIndexCompose<usize>,
        Self::Lens: 'static,
    {
        let len = ProjectSlice::len(self);
        let this = self.clone();
        (0..len).map(move |i| this.clone().index(i))
    }

    /// Try to get the item at `index` as a projection.
    fn get(
        self,
        index: usize,
    ) -> Option<
        Self::Rebind<
            <<Self::Lens as Readable>::Target as Index<usize>>::Output,
            IndexWrite<usize, Self::Lens>,
        >,
    >
    where
        Self: ProjectIndex<usize> + ProjectIndexCompose<usize>,
        Self::Lens: 'static,
    {
        if index >= ProjectSlice::len(&self) {
            None
        } else {
            Some(self.index(index))
        }
    }
}

impl<T: 'static, P> ProjectSlice<T> for P where P: Project<Lens: Readable<Target = Vec<T>>> {}

/// Mutation methods on vector-shaped projections.
pub trait ProjectVec<T: 'static>: ProjectLens + ProjectReact
where
    Self::Lens: Writable<Target = Vec<T>>,
{
    /// Push an item to the end.
    fn push(&self, value: T) {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().push(value);
    }

    /// Remove and return the item at `index`.
    fn remove(&self, index: usize) -> T {
        self.project_mark_dirty_shallow();
        self.project_mark_dirty_at_and_after_index(index);
        self.project_lens().write_unchecked().remove(index)
    }

    /// Insert an item at `index`.
    fn insert(&self, index: usize, value: T) {
        self.project_mark_dirty_shallow();
        self.project_mark_dirty_at_and_after_index(index);
        self.project_lens().write_unchecked().insert(index, value);
    }

    /// Clear all items.
    fn clear(&self) {
        self.project_mark_dirty();
        self.project_lens().write_unchecked().clear();
    }

    /// Retain only elements for which `f` returns true.
    fn retain(&self, mut f: impl FnMut(&T) -> bool) {
        let mut index = 0;
        let mut first_removed_index: Option<usize> = None;
        self.project_lens().write_unchecked().retain(|item| {
            let keep = f(item);
            if !keep {
                first_removed_index = first_removed_index.or(Some(index));
            }
            index += 1;
            keep
        });
        if let Some(index) = first_removed_index {
            self.project_mark_dirty_shallow();
            self.project_mark_dirty_at_and_after_index(index);
        }
    }
}

impl<T: 'static, P> ProjectVec<T> for P
where
    P: ProjectLens + ProjectReact,
    P::Lens: Writable<Target = Vec<T>>,
{
}

#[doc(hidden)]
pub trait ProjectHashMapGetCompose<Q, K: 'static, V: 'static, St: 'static>: Project
where
    Q: Hash + Eq + 'static,
    K: Borrow<Q> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
    Self::Lens: Readable<Target = HashMap<K, V, St>> + 'static,
{
    fn project_hashmap_get(self, key: Q) -> Self::Rebind<V, HashMapGetWrite<Q, Self::Lens>>;
}

impl<Q, K, V, St, P, Lens> ProjectHashMapGetCompose<Q, K, V, St> for P
where
    K: 'static + Borrow<Q> + Eq + Hash,
    V: 'static,
    St: 'static + BuildHasher,
    Q: Hash + Eq + 'static,
    P: Project<Lens = Lens> + ProjectCompose<V, HashMapGetWrite<Q, Lens>>,
    Lens: Readable<Target = HashMap<K, V, St>> + 'static,
{
    fn project_hashmap_get(self, key: Q) -> Self::Rebind<V, HashMapGetWrite<Q, Self::Lens>> {
        let created = Location::caller();
        self.project_hash_key(key.borrow())
            .project_compose(|lens| HashMapGetWrite::new(key, lens, created))
    }
}

/// Read-side methods on `HashMap<K, V, St>` projections.
pub trait ProjectHashMap<K: 'static, V: 'static, St: 'static>: Project
where
    Self::Lens: Readable<Target = HashMap<K, V, St>>,
{
    /// Map length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().len()
    }

    /// Is the map empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_empty()
    }

    /// Check whether a key exists; tracks shallowly.
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().contains_key(key)
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
        Self: ProjectHashMapGetCompose<K, K, V, St>,
    {
        ProjectHashMap::<K, V, St>::len(self);
        let keys: Vec<_> = self
            .project_lens()
            .peek_unchecked()
            .keys()
            .cloned()
            .collect();
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
        Self: ProjectHashMapGetCompose<K, K, V, St>,
    {
        ProjectHashMap::<K, V, St>::len(self);
        let keys = self
            .project_lens()
            .peek_unchecked()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
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
        Self: ProjectHashMapGetCompose<Q, K, V, St>,
    {
        self.contains_key(&key).then(|| {
            <Self as ProjectHashMapGetCompose<Q, K, V, St>>::project_hashmap_get(self, key)
        })
    }

    /// Get a projection for the value at `key` without existence check.
    #[track_caller]
    fn get_unchecked<Q>(self, key: Q) -> Self::Rebind<V, HashMapGetWrite<Q, Self::Lens>>
    where
        Q: Hash + Eq + 'static,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
        Self::Lens: 'static,
        Self: ProjectHashMapGetCompose<Q, K, V, St>,
    {
        <Self as ProjectHashMapGetCompose<Q, K, V, St>>::project_hashmap_get(self, key)
    }
}

impl<K, V, St, P> ProjectHashMap<K, V, St> for P
where
    K: 'static,
    V: 'static,
    St: 'static,
    P: Project,
    P::Lens: Readable<Target = HashMap<K, V, St>>,
{
}

/// Mutation methods on `HashMap<K, V, St>` projections.
pub trait ProjectHashMapMut<K: 'static, V: 'static, St: 'static>: Project
where
    Self::Lens: Writable<Target = HashMap<K, V, St>>,
{
    /// Insert a key-value pair; marks shape dirty + existing child at the hash path dirty.
    fn insert(&self, key: K, value: V)
    where
        K: Eq + Hash,
        St: BuildHasher,
        Self: Clone,
    {
        self.project_mark_dirty_shallow();
        self.clone().project_hash_key(&key).project_mark_dirty();
        self.project_lens().write_unchecked().insert(key, value);
    }

    /// Remove a key; marks shape dirty.
    fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().remove(key)
    }

    /// Clear the map; marks shape dirty.
    fn clear(&self) {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().clear();
    }

    /// Retain only entries matching `f`; marks shape dirty.
    fn retain(&self, mut f: impl FnMut(&K, &V) -> bool) {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().retain(|k, v| f(k, v));
    }
}

impl<K, V, St, P> ProjectHashMapMut<K, V, St> for P
where
    K: 'static,
    V: 'static,
    St: 'static,
    P: Project,
    P::Lens: Writable<Target = HashMap<K, V, St>>,
{
}

#[doc(hidden)]
pub trait ProjectBTreeMapGetCompose<Q, K: 'static, V: 'static>: Project
where
    Q: Hash + Ord + 'static,
    K: Borrow<Q> + Ord + 'static,
    Self::Lens: Readable<Target = BTreeMap<K, V>> + 'static,
{
    fn project_btreemap_get(self, key: Q) -> Self::Rebind<V, BTreeMapGetWrite<Q, Self::Lens>>;
}

impl<Q, K, V, P, Lens> ProjectBTreeMapGetCompose<Q, K, V> for P
where
    K: 'static + Borrow<Q> + Ord,
    V: 'static,
    Q: Hash + Ord + 'static,
    P: Project<Lens = Lens> + ProjectCompose<V, BTreeMapGetWrite<Q, Lens>>,
    Lens: Readable<Target = BTreeMap<K, V>> + 'static,
{
    fn project_btreemap_get(self, key: Q) -> Self::Rebind<V, BTreeMapGetWrite<Q, Self::Lens>> {
        let created = Location::caller();
        self.project_hash_key(key.borrow())
            .project_compose(|lens| BTreeMapGetWrite::new(key, lens, created))
    }
}

/// Read-side methods on `BTreeMap<K, V>` projections.
pub trait ProjectBTreeMap<K: 'static, V: 'static>: Project
where
    Self::Lens: Readable<Target = BTreeMap<K, V>>,
{
    /// Map length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().len()
    }

    /// Is the map empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_empty()
    }

    /// Check whether a key exists; tracks shallowly.
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Ord,
        K: Borrow<Q> + Ord,
    {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().contains_key(key)
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
        Self: ProjectBTreeMapGetCompose<K, K, V>,
    {
        ProjectBTreeMap::<K, V>::len(self);
        let keys: Vec<_> = self
            .project_lens()
            .peek_unchecked()
            .keys()
            .cloned()
            .collect();
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
        Self: ProjectBTreeMapGetCompose<K, K, V>,
    {
        ProjectBTreeMap::<K, V>::len(self);
        let keys = self
            .project_lens()
            .peek_unchecked()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
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
        Self: ProjectBTreeMapGetCompose<Q, K, V>,
    {
        self.contains_key(&key)
            .then(|| <Self as ProjectBTreeMapGetCompose<Q, K, V>>::project_btreemap_get(self, key))
    }

    /// Get a projection for the value at `key` without checking existence.
    #[track_caller]
    fn get_unchecked<Q>(self, key: Q) -> Self::Rebind<V, BTreeMapGetWrite<Q, Self::Lens>>
    where
        Q: Hash + Ord + 'static,
        K: Borrow<Q> + Ord,
        Self::Lens: 'static,
        Self: ProjectBTreeMapGetCompose<Q, K, V>,
    {
        <Self as ProjectBTreeMapGetCompose<Q, K, V>>::project_btreemap_get(self, key)
    }
}

impl<K, V, P> ProjectBTreeMap<K, V> for P
where
    K: 'static,
    V: 'static,
    P: Project,
    P::Lens: Readable<Target = BTreeMap<K, V>>,
{
}

/// Mutation methods on `BTreeMap<K, V>` projections.
pub trait ProjectBTreeMapMut<K: 'static, V: 'static>: Project
where
    Self::Lens: Writable<Target = BTreeMap<K, V>>,
{
    /// Insert a key-value pair; marks shape dirty.
    fn insert(&self, key: K, value: V) -> Option<V>
    where
        K: Ord + Hash,
        Self: Clone,
    {
        self.project_mark_dirty_shallow();
        self.clone().project_hash_key(&key).project_mark_dirty();
        self.project_lens().write_unchecked().insert(key, value)
    }

    /// Remove a key; marks shape dirty.
    fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Ord,
        K: Borrow<Q> + Ord,
    {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().remove(key)
    }

    /// Clear the map; marks shape dirty.
    fn clear(&self) {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().clear();
    }

    /// Retain only entries matching `f`; marks shape dirty.
    fn retain(&self, mut f: impl FnMut(&K, &mut V) -> bool)
    where
        K: Ord,
    {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().retain(|k, v| f(k, v));
    }
}

impl<K, V, P> ProjectBTreeMapMut<K, V> for P
where
    K: 'static,
    V: 'static,
    P: Project,
    P::Lens: Writable<Target = BTreeMap<K, V>>,
{
}

/// A carrier that exposes its current/eventual value as a future.
///
/// This is the async sibling of [`ProjectLens`]: where `ProjectLens` gives you
/// a synchronous, subscribable view, `ProjectAwait` gives you a one-shot future
/// that resolves to the carrier's value. Implementors decide what "resolved"
/// means — for resources it's "the backing task finished".
pub trait ProjectAwait {
    /// The resolved value yielded by the future.
    type Output;
    /// The future returned by [`project_future`](Self::project_future).
    type Future: Future<Output = Self::Output>;
    /// Build a future that resolves to the carrier's value.
    fn project_future(self) -> Self::Future;
}

/// Future adapter that projects its parent future's output through `map` at
/// resolve time. This is the awaited-side analogue of
/// [`MappedMutSignal`](crate::MappedMutSignal).
pub struct FutureProject<Fut, S, U: ?Sized> {
    future: Fut,
    map: fn(&S) -> &U,
}

impl<Fut, S, U: ?Sized> FutureProject<Fut, S, U> {
    /// Wrap `future` with a projection through `map` applied at resolve time.
    pub fn new(future: Fut, map: fn(&S) -> &U) -> Self {
        Self { future, map }
    }
}

impl<Fut, S, U> Future for FutureProject<Fut, S, U>
where
    Fut: Future<Output = S> + Unpin,
    U: Clone,
{
    type Output = U;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<U> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(s) => Poll::Ready((this.map)(&s).clone()),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future adapter that flattens `Option<Option<T>>` into `Option<T>` at
/// resolve time.
pub struct FlattenSomeFuture<Fut> {
    future: Fut,
}

impl<Fut> FlattenSomeFuture<Fut> {
    /// Wrap `future` with an `Option<Option<_>>::flatten` applied at resolve time.
    pub fn new(future: Fut) -> Self {
        Self { future }
    }
}

impl<Fut, X> Future for FlattenSomeFuture<Fut>
where
    Fut: Future<Output = Option<Option<X>>> + Unpin,
{
    type Output = Option<X>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<X>> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(v) => Poll::Ready(v.flatten()),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Convenience methods for [`ProjectAwait`] carriers.
pub trait ProjectAwaitExt: ProjectAwait + Sized
where
    Self::Future: Unpin,
{
    /// Project the resolved value through `map` at await time.
    fn project_await_map<U>(
        self,
        map: fn(&Self::Output) -> &U,
    ) -> FutureProject<Self::Future, Self::Output, U>
    where
        U: Clone,
    {
        FutureProject::new(self.project_future(), map)
    }
}

impl<T> ProjectAwaitExt for T
where
    T: ProjectAwait,
    T::Future: Unpin,
{
}

/// Convenience methods for [`ProjectAwait`] carriers whose output is
/// `Option<Option<T>>`.
pub trait ProjectAwaitFlattenExt<T>: ProjectAwait<Output = Option<Option<T>>> + Sized
where
    Self::Future: Unpin,
{
    /// Flatten the resolved `Option<Option<T>>` into `Option<T>` at await time.
    fn project_await_flatten(self) -> FlattenSomeFuture<Self::Future> {
        FlattenSomeFuture::new(self.project_future())
    }
}

impl<P, T> ProjectAwaitFlattenExt<T> for P
where
    P: ProjectAwait<Output = Option<Option<T>>>,
    P::Future: Unpin,
{
}

/// Forward [`ProjectAwait`] through a [`MappedMutSignal`] by composing the
/// read-projection fn into a [`FutureProject`] adapter at await time.
///
/// This makes any lens chain produced by [`ProjectLensExt::project_map`] /
/// [`ProjectPathExt::project_child`] over an awaitable parent automatically
/// awaitable as well.
impl<O, V, FMut> ProjectAwait for MappedMutSignal<O, V, fn(&<V as Readable>::Target) -> &O, FMut>
where
    V: Readable + ProjectAwait<Output = <V as Readable>::Target>,
    V::Future: Unpin,
    <V as Readable>::Target: Sized + 'static,
    O: Clone + 'static,
{
    type Output = O;
    type Future = FutureProject<V::Future, <V as Readable>::Target, O>;
    fn project_future(self) -> Self::Future {
        let (value, map_fn, _) = self.into_parts();
        FutureProject::new(value.project_future(), map_fn)
    }
}

/// Future adapter that indexes into the resolved value at await time.
pub struct IndexFutureProject<Fut, Idx> {
    future: Fut,
    index: Idx,
}

impl<Fut, Idx> IndexFutureProject<Fut, Idx> {
    /// Wrap `future` with an `Index<Idx>` projection applied at resolve time.
    pub fn new(future: Fut, index: Idx) -> Self {
        Self { future, index }
    }
}

impl<Fut, Idx, S, U> Future for IndexFutureProject<Fut, Idx>
where
    Fut: Future<Output = S> + Unpin,
    Idx: Clone + Unpin,
    S: Index<Idx, Output = U>,
    U: Clone,
{
    type Output = U;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<U> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(s) => Poll::Ready(s[this.index.clone()].clone()),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Forward [`ProjectAwait`] through an [`IndexWrite`] by indexing the resolved
/// value at await time.
impl<Idx, Write, S, U> ProjectAwait for IndexWrite<Idx, Write>
where
    Write: ProjectAwait<Output = S>,
    Write::Future: Unpin,
    Idx: Clone + Unpin + 'static,
    S: Index<Idx, Output = U> + 'static,
    U: Clone + Sized + 'static,
{
    type Output = U;
    type Future = IndexFutureProject<Write::Future, Idx>;
    fn project_future(self) -> Self::Future {
        IndexFutureProject::new(self.write.project_future(), self.index)
    }
}

/// Future adapter that looks up a key in the resolved `HashMap` at await time.
pub struct HashMapGetFutureProject<Fut, Q> {
    future: Fut,
    key: Q,
    created: &'static Location<'static>,
}

impl<Fut, Q> HashMapGetFutureProject<Fut, Q> {
    /// Wrap `future` with a `HashMap::get(&key)` projection applied at resolve
    /// time. Panics on resolve if the key is missing.
    pub fn new(future: Fut, key: Q, created: &'static Location<'static>) -> Self {
        Self {
            future,
            key,
            created,
        }
    }
}

impl<Fut, Q, K, V, St> Future for HashMapGetFutureProject<Fut, Q>
where
    Fut: Future<Output = HashMap<K, V, St>> + Unpin,
    Q: Hash + Eq + Unpin,
    K: Borrow<Q> + Eq + Hash,
    St: BuildHasher,
    V: Clone,
{
    type Output = V;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<V> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(map) => match map.get(&this.key) {
                Some(v) => Poll::Ready(v.clone()),
                None => panic!(
                    "HashMap key not present at resolve time (projection created at {})",
                    this.created
                ),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Forward [`ProjectAwait`] through a [`HashMapGetWrite`] by looking up the
/// captured key in the resolved map at await time.
impl<Q, Write, K, V, St> ProjectAwait for HashMapGetWrite<Q, Write>
where
    Write: ProjectAwait<Output = HashMap<K, V, St>>,
    Write::Future: Unpin,
    Q: Hash + Eq + Unpin + 'static,
    K: Borrow<Q> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
    V: Clone + 'static,
{
    type Output = V;
    type Future = HashMapGetFutureProject<Write::Future, Q>;
    fn project_future(self) -> Self::Future {
        HashMapGetFutureProject::new(self.write.project_future(), self.index, self.created)
    }
}

/// Future adapter that looks up a key in the resolved `BTreeMap` at await time.
pub struct BTreeMapGetFutureProject<Fut, Q> {
    future: Fut,
    key: Q,
    created: &'static Location<'static>,
}

impl<Fut, Q> BTreeMapGetFutureProject<Fut, Q> {
    /// Wrap `future` with a `BTreeMap::get(&key)` projection applied at resolve
    /// time. Panics on resolve if the key is missing.
    pub fn new(future: Fut, key: Q, created: &'static Location<'static>) -> Self {
        Self {
            future,
            key,
            created,
        }
    }
}

impl<Fut, Q, K, V> Future for BTreeMapGetFutureProject<Fut, Q>
where
    Fut: Future<Output = BTreeMap<K, V>> + Unpin,
    Q: Ord + Unpin,
    K: Borrow<Q> + Ord,
    V: Clone,
{
    type Output = V;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<V> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(map) => match map.get(&this.key) {
                Some(v) => Poll::Ready(v.clone()),
                None => panic!(
                    "BTreeMap key not present at resolve time (projection created at {})",
                    this.created
                ),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Forward [`ProjectAwait`] through a [`BTreeMapGetWrite`] by looking up the
/// captured key in the resolved map at await time.
impl<Q, Write, K, V> ProjectAwait for BTreeMapGetWrite<Q, Write>
where
    Write: ProjectAwait<Output = BTreeMap<K, V>>,
    Write::Future: Unpin,
    Q: Ord + Unpin + 'static,
    K: Borrow<Q> + Ord + 'static,
    V: Clone + 'static,
{
    type Output = V;
    type Future = BTreeMapGetFutureProject<Write::Future, Q>;
    fn project_future(self) -> Self::Future {
        BTreeMapGetFutureProject::new(self.write.project_future(), self.index, self.created)
    }
}
