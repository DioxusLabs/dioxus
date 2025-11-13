//! This module contains the `SelectorScope` type with raw access to the underlying store system. Most applications should
//! use the [`Store`](dioxus_stores_macro::Store) macro to derive stores for their data structures, which provides a more ergonomic API.

use std::{fmt::Debug, hash::Hash};

use crate::subscriptions::{PathKey, StoreSubscriptions, TinyVec};
use dioxus_core::Subscribers;
use dioxus_signals::{
    BorrowError, BorrowMutError, MappedMutSignal, Readable, ReadableRef, Writable, WritableExt,
    WritableRef,
};

/// SelectorScope is the primitive that backs the store system.
///
/// Under the hood stores consist of two different parts:
/// - The underlying lock that contains the data in the store.
/// - A tree of subscriptions used to make the store reactive.
///
/// The `SelectorScope` contains a view into the lock (`Lens`) and a path into the subscription tree. When
/// the selector is read to, it will track the current path in the subscription tree. When it it written to
/// it marks itself and all its children as dirty.
///
/// When you derive the [`Store`](dioxus_stores_macro::Store) macro on your data structure,
/// it generates methods that map the lock to a new type and scope the path to a specific part of the subscription structure.
/// For example, a `Counter` store might look like this:
///
/// ```rust, ignore
/// #[derive(Store)]
/// struct Counter {
///     count: i32,
/// }
///
/// impl CounterStoreExt for Store<Counter> {
///     fn count(
///         self,
///     ) -> dioxus_stores::Store<
///         i32,
///         dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<i32, __W>,
///     > {
///         let __map_field: fn(&CounterTree) -> &i32 = |value| &value.count;
///         let __map_mut_field: fn(&mut CounterTree) -> &mut i32 = |value| &mut value.count;
///         let scope = self.selector().scope(0u32, __map_field, __map_mut_field);
///         dioxus_stores::Store::new(scope)
///     }
/// }
/// ```
///
/// The `count` method maps the lock to the `i32` type and creates a child `0` path in the subscription tree. Only writes
/// to that `0` path or its parents will trigger a re-render of the components that read the `count` field.
#[derive(PartialEq)]
pub struct SelectorScope<Lens> {
    path: TinyVec,
    store: StoreSubscriptions,
    write: Lens,
}

impl<Lens> Debug for SelectorScope<Lens> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("SelectorScope")
            .field("path", &self.path)
            .finish()
    }
}

impl<Lens> Clone for SelectorScope<Lens>
where
    Lens: Clone,
{
    fn clone(&self) -> Self {
        Self {
            path: self.path,
            store: self.store,
            write: self.write.clone(),
        }
    }
}

impl<Lens> Copy for SelectorScope<Lens> where Lens: Copy {}

impl<Lens> SelectorScope<Lens> {
    pub(crate) fn new(path: TinyVec, store: StoreSubscriptions, write: Lens) -> Self {
        Self { path, store, write }
    }

    /// Create a child selector scope for a hash key. The scope will only be marked as dirty when a
    /// write occurs to that key or its parents.
    ///
    /// Note the hash is lossy, so there may rarely be collisions. If a collision does occur, it may
    /// cause reruns in a part of the app that has not changed. As long as derived data is pure,
    /// this should not cause issues.
    pub fn hash_child<U: ?Sized, T, F, FMut>(
        self,
        index: &impl Hash,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, Lens, F, FMut>>
    where
        F: Fn(&T) -> &U,
        FMut: Fn(&mut T) -> &mut U,
    {
        let hash = self.store.hash(index);
        self.child(hash, map, map_mut)
    }

    /// Create a child selector scope for a specific index. The scope will only be marked as dirty when a
    /// write occurs to that index or its parents.
    pub fn child<U: ?Sized, T, F, FMut>(
        self,
        index: PathKey,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, Lens, F, FMut>>
    where
        F: Fn(&T) -> &U,
        FMut: Fn(&mut T) -> &mut U,
    {
        self.child_unmapped(index).map(map, map_mut)
    }

    /// Create a hashed child selector scope for a specific index without mapping the writer. The scope will only
    /// be marked as dirty when a write occurs to that index or its parents.
    pub fn hash_child_unmapped(self, index: &impl Hash) -> SelectorScope<Lens> {
        let hash = self.store.hash(index);
        self.child_unmapped(hash)
    }

    /// Create a child selector scope for a specific index without mapping the writer. The scope will only
    /// be marked as dirty when a write occurs to that index or its parents.
    pub fn child_unmapped(mut self, index: PathKey) -> SelectorScope<Lens> {
        self.path.push(index);
        self
    }

    /// Map the view into the writable data without creating a child selector scope
    pub fn map<U: ?Sized, T, F, FMut>(
        self,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, Lens, F, FMut>>
    where
        F: Fn(&T) -> &U,
        FMut: Fn(&mut T) -> &mut U,
    {
        self.map_writer(move |write| MappedMutSignal::new(write, map, map_mut))
    }

    /// Track this scope shallowly.
    pub fn track_shallow(&self) {
        self.store.track(&self.path);
    }

    /// Track this scope recursively.
    pub fn track(&self) {
        self.store.track_recursive(&self.path);
    }

    /// Mark this scope as dirty recursively.
    pub fn mark_dirty(&self) {
        self.store.mark_dirty(&self.path);
    }

    /// Mark this scope as dirty shallowly.
    pub fn mark_dirty_shallow(&self) {
        self.store.mark_dirty_shallow(&self.path);
    }

    /// Mark this scope as dirty at and after the given index.
    pub fn mark_dirty_at_and_after_index(&self, index: usize) {
        self.store.mark_dirty_at_and_after_index(&self.path, index);
    }

    /// Map the writer to a new type.
    pub fn map_writer<W2>(self, map: impl FnOnce(Lens) -> W2) -> SelectorScope<W2> {
        SelectorScope {
            path: self.path,
            store: self.store,
            write: map(self.write),
        }
    }

    /// Write without notifying subscribers.
    pub fn write_untracked(&self) -> WritableRef<'static, Lens>
    where
        Lens: Writable,
    {
        self.write.write_unchecked()
    }

    /// Borrow the writer
    pub(crate) fn as_ref(&self) -> SelectorScope<&Lens> {
        SelectorScope {
            path: self.path,
            store: self.store,
            write: &self.write,
        }
    }
}

impl<Lens: Readable> Readable for SelectorScope<Lens> {
    type Target = Lens::Target;
    type Storage = Lens::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Lens>, BorrowError> {
        self.track();
        self.write.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Lens>, BorrowError> {
        self.write.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers {
        self.store.subscribers(&self.path)
    }
}

impl<Lens: Writable> Writable for SelectorScope<Lens> {
    type WriteMetadata = Lens::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Lens>, BorrowMutError> {
        self.mark_dirty();
        self.write.try_write_unchecked()
    }
}
