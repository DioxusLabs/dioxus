use std::{fmt::Debug, hash::Hash};

use crate::subscriptions::{StoreSubscriptions, TinyVec};
use dioxus_core::Subscribers;
use dioxus_signals::{
    BorrowError, BorrowMutError, MappedMutSignal, Readable, ReadableRef, Writable, WritableExt,
    WritableRef,
};

pub struct SelectorScope<W> {
    path: TinyVec,
    store: StoreSubscriptions,
    write: W,
}

impl<W> Debug for SelectorScope<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("SelectorScope")
            .field("path", &self.path)
            .finish()
    }
}

impl<W: PartialEq> PartialEq for SelectorScope<W> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.write == other.write
    }
}

impl<W> Clone for SelectorScope<W>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            path: self.path,
            store: self.store,
            write: self.write.clone(),
        }
    }
}

impl<W> Copy for SelectorScope<W> where W: Copy {}

impl<W> SelectorScope<W> {
    pub(crate) fn new(path: TinyVec, store: StoreSubscriptions, write: W) -> Self {
        Self { path, store, write }
    }

    pub fn hash_child<U: ?Sized, F, FMut>(
        self,
        index: impl Hash,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, W, F, FMut>>
    where
        W: Writable + 'static,
        F: Fn(&W::Target) -> &U + 'static,
        FMut: Fn(&mut W::Target) -> &mut U + 'static,
    {
        let hash = self.store.hash(index);
        self.child(hash, map, map_mut)
    }

    pub fn child<U: ?Sized, F, FMut>(
        mut self,
        index: u32,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, W, F, FMut>>
    where
        W: Writable + 'static,
        F: Fn(&W::Target) -> &U + 'static,
        FMut: Fn(&mut W::Target) -> &mut U + 'static,
    {
        self.path.push(index);
        self.map(map, map_mut)
    }

    pub fn map<U: ?Sized, F, FMut>(
        self,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, W, F, FMut>>
    where
        W: Writable,
        F: Fn(&W::Target) -> &U + 'static,
        FMut: Fn(&mut W::Target) -> &mut U + 'static,
    {
        let write = self.write.map_mut(map, map_mut);
        SelectorScope::new(self.path, self.store, write)
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
    pub fn map_writer<W2>(self, map: impl FnOnce(W) -> W2) -> SelectorScope<W2> {
        SelectorScope {
            path: self.path,
            store: self.store,
            write: map(self.write),
        }
    }

    /// Write without notifying subscribers.
    pub fn write_untracked(&self) -> WritableRef<'static, W>
    where
        W: Writable,
    {
        self.write.write_unchecked()
    }
}

impl<W: Readable> Readable for SelectorScope<W> {
    type Target = W::Target;
    type Storage = W::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.track();
        self.write.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.write.try_peek_unchecked()
    }

    fn subscribers(&self) -> Option<Subscribers> {
        Some(self.store.subscribers(&self.path))
    }
}

impl<W: Writable> Writable for SelectorScope<W> {
    type WriteMetadata = W::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, W>, BorrowMutError> {
        self.mark_dirty();
        self.write.try_write_unchecked()
    }
}
