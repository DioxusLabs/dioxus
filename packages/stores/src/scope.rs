//! This module contains the `SelectorScope` type with raw access to the underlying store system. Most applications should
//! use the [`Store`](dioxus_stores_macro::Store) macro to derive stores for their data structures, which provides a more ergonomic API.

use std::{fmt::Debug, hash::Hash};

use dioxus_core::Subscribers;
use dioxus_optics::{Combinator, LensOp, PathSegment, SubscriptionTree};
use dioxus_signals::{
    BorrowError, BorrowMutError, CopyValue, Readable, ReadableExt, ReadableRef, SyncStorage,
    Writable, WritableExt, WritableRef,
};

/// SelectorScope is the primitive that backs the store system.
///
/// Under the hood stores consist of two different parts:
/// - The underlying lock that contains the data in the store.
/// - A tree of subscriptions used to make the store reactive.
///
/// The `SelectorScope` contains a view into the lock (`Lens`) and a path into the subscription tree. When
/// the selector is read to, it will track the current path in the subscription tree. When it is written to
/// it marks itself and all its children as dirty.
///
/// The subscription tree and path machinery are both provided by
/// [`dioxus_optics`](dioxus_optics). A `SelectorScope` is effectively a
/// store-flavored adapter that wires the `Readable` / `Writable` signal
/// traits to the optics path-subscription system.
///
/// The path itself lives behind a [`CopyValue`] so the whole `SelectorScope`
/// stays `Copy` whenever its `Lens` is — preserving the ergonomic of moving
/// stores into closures by-copy that downstream callers (the macro,
/// `use_resource`, `UseWebsocket`, etc.) rely on.
pub struct SelectorScope<Lens> {
    path: CopyValue<Vec<PathSegment>, SyncStorage>,
    store: CopyValue<SubscriptionTree, SyncStorage>,
    write: Lens,
}

impl<Lens: Clone> Clone for SelectorScope<Lens> {
    fn clone(&self) -> Self {
        Self {
            path: self.path,
            store: self.store,
            write: self.write.clone(),
        }
    }
}

impl<Lens: Copy> Copy for SelectorScope<Lens> {}

impl<Lens: PartialEq> PartialEq for SelectorScope<Lens> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.write == other.write
    }
}

impl<Lens> Debug for SelectorScope<Lens> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("SelectorScope")
            .field("path", &*self.path.read())
            .finish()
    }
}

impl<Lens> SelectorScope<Lens> {
    pub(crate) fn new(path: Vec<PathSegment>, store: SubscriptionTree, write: Lens) -> Self {
        Self {
            path: CopyValue::new_maybe_sync(path),
            store: CopyValue::new_maybe_sync(store),
            write,
        }
    }

    fn with_path<R>(&self, f: impl FnOnce(&[PathSegment]) -> R) -> R {
        let read = self.path.read();
        f(read.as_slice())
    }

    /// Create a child selector scope whose lens is an optics
    /// [`LensOp`]-backed [`Combinator`] over the parent. This is the shape
    /// the `#[derive(Store)]` macro emits, so generated `.field()` methods
    /// produce optic-backed lenses end-to-end.
    pub fn child_with_optic<T, U>(
        self,
        segment: PathSegment,
        read: fn(&T) -> &U,
        write: fn(&mut T) -> &mut U,
    ) -> SelectorScope<Combinator<Lens, LensOp<T, U>>>
    where
        T: 'static,
        U: 'static,
    {
        self.child_unmapped(segment)
            .map_writer(|w| Combinator::new(w, LensOp::new(read, write)))
    }

    /// Hash an arbitrary key into this scope's path space.
    pub fn hash_key(&self, index: &(impl Hash + ?Sized)) -> PathSegment {
        PathSegment::hashed(index)
    }

    /// Extend this scope's path with one segment without re-mapping the writer.
    pub fn child_unmapped(self, segment: PathSegment) -> SelectorScope<Lens> {
        let new_path: Vec<PathSegment> = {
            let read = self.path.read();
            let mut next = Vec::with_capacity(read.len() + 1);
            next.extend_from_slice(&read);
            next.push(segment);
            next
        };
        SelectorScope {
            path: CopyValue::new_maybe_sync(new_path),
            store: self.store,
            write: self.write,
        }
    }

    /// Track this scope shallowly.
    pub fn track_shallow(&self) {
        self.with_path(|p| self.store.read().track(p));
    }

    /// Track this scope recursively (deep subscription).
    pub fn track(&self) {
        self.with_path(|p| self.store.read().track_deep(p));
    }

    /// Mark this scope as dirty recursively.
    pub fn mark_dirty(&self) {
        self.with_path(|p| self.store.read().notify(p));
    }

    /// Mark this scope as dirty shallowly (no descendants).
    pub fn mark_dirty_shallow(&self) {
        self.with_path(|p| self.store.read().notify_node(p));
    }

    /// Mark every child of this scope whose numeric index is `>= index` dirty.
    pub fn mark_dirty_at_and_after_index(&self, index: usize) {
        self.with_path(|p| self.store.read().notify_from(p, index as u64));
    }

    /// Borrow the lens/writer.
    pub fn writer(&self) -> &Lens {
        &self.write
    }

    /// Snapshot the underlying subscription tree for this scope.
    pub fn subscription_tree(&self) -> SubscriptionTree {
        self.store.read().clone()
    }

    /// Run a closure with the scope's accumulated path segments.
    pub fn with_path_segments<R>(&self, f: impl FnOnce(&[PathSegment]) -> R) -> R {
        self.with_path(f)
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
}

impl<Lens: Readable> Readable for SelectorScope<Lens> {
    type Target = Lens::Target;
    type Storage = Lens::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Lens>, BorrowError> {
        // `self.track()` subscribes the current reactive context at this
        // scope's path. Use `try_peek_unchecked` on the inner so any nested
        // `SelectorScope` / `Subscribed` it wraps does **not** additionally
        // subscribe at its own (broader) path. Otherwise a chain like
        // `Store<T, Combinator<Store<X>, LensOp<X, T>>>` would subscribe at
        // both the leaf field path *and* the root path — making every sibling
        // of the leaf wake on a root write.
        self.track();
        self.write.try_peek_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Lens>, BorrowError> {
        self.write.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers {
        self.with_path(|p| self.store.read().shallow_subscribers(p))
    }
}

impl<Lens: Writable> Writable for SelectorScope<Lens> {
    type WriteMetadata = Lens::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Lens>, BorrowMutError> {
        self.mark_dirty();
        self.write.try_write_unchecked()
    }

    fn try_write_silent(&self) -> Result<WritableRef<'static, Lens>, BorrowMutError> {
        // Silent write: skip `mark_dirty` at our scope path. Caller (usually
        // a wrapping `Subscribed` that fires at a leaf path) is responsible
        // for notifying subscribers; firing here would wake every
        // descendant of this scope path too.
        self.write.try_write_unchecked()
    }
}
