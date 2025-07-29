#![allow(clippy::type_complexity)]

use std::hash::Hash;

use crate::subscriptions::{StoreSubscriptions, TinyVec};
use dioxus_core::{use_hook, Subscribers};
use dioxus_signals::{
    BorrowError, BorrowMutError, CopyValue, MappedMutSignal, Readable, ReadableRef, Storage,
    UnsyncStorage, Writable, WritableExt, WritableRef, WriteSignal,
};

mod foreign;
pub use foreign::*;
mod builtin;
pub use builtin::*;
mod impls;
mod subscriptions;

// Re-exported for the macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_core;
    pub use dioxus_signals;
}

pub struct SelectorScope<W> {
    path: TinyVec,
    store: StoreSubscriptions,
    write: W,
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
    fn new(path: TinyVec, store: StoreSubscriptions, write: W) -> Self {
        Self { path, store, write }
    }

    pub fn hash_scope<U: ?Sized, F, FMut>(
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
        self.scope(hash, map, map_mut)
    }

    pub fn scope<U: ?Sized, F, FMut>(
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
        self.scope_raw(map, map_mut)
    }

    pub fn scope_raw<U: ?Sized, F, FMut>(
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

    fn track(&self) {
        self.store.track(&self.path);
    }

    fn track_recursive(&self) {
        self.store.track_recursive(&self.path);
    }

    fn mark_dirty(&self) {
        self.store.mark_dirty(&self.path);
    }

    fn mark_dirty_shallow(&self) {
        self.store.mark_dirty_shallow(&self.path);
    }

    fn mark_dirty_at_and_after_index(&self, index: usize) {
        self.store.mark_dirty_at_and_after_index(&self.path, index);
    }

    /// Map the writer to a new type.
    pub fn map<W2>(self, map: impl FnOnce(W) -> W2) -> SelectorScope<W2> {
        SelectorScope {
            path: self.path,
            store: self.store,
            write: map(self.write),
        }
    }
}

impl<W: Readable> SelectorScope<W> {
    pub fn try_read_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.track_recursive();
        self.write.try_read_unchecked()
    }

    pub fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.write.try_peek_unchecked()
    }

    pub fn subscribers(&self) -> Option<Subscribers> {
        Some(self.store.subscribers(&self.path))
    }
}

impl<W: Writable> SelectorScope<W> {
    pub fn try_write_unchecked(&self) -> Result<WritableRef<'static, W>, BorrowMutError> {
        self.mark_dirty();
        self.write.try_write_unchecked()
    }
}

pub type Store<T, W = WriteSignal<T>> = <T as Storable>::Store<W>;

pub fn create_maybe_sync_store<T: Storable, S: Storage<T>>(
    value: T,
) -> Store<T, MappedMutSignal<T, CopyValue<T, S>>> {
    let store = StoreSubscriptions::new();
    let value = CopyValue::new_maybe_sync(value);

    let path = TinyVec::new();
    let map: fn(&T) -> &T = |value| value;
    let map_mut: fn(&mut T) -> &mut T = |value| value;
    let selector = SelectorScope {
        path,
        store,
        write: value.map_mut(map, map_mut),
    };
    T::create_selector(selector)
}

pub fn use_maybe_sync_store<T: Storable, S: Storage<T>>(
    init: impl Fn() -> T,
) -> Store<T, MappedMutSignal<T, CopyValue<T, S>>>
where
    Store<T, MappedMutSignal<T, CopyValue<T, S>>>: Clone,
{
    use_hook(move || create_maybe_sync_store(init()))
}

pub fn create_store<T: Storable>(value: T) -> Store<T, MappedMutSignal<T, CopyValue<T>>> {
    create_maybe_sync_store::<T, UnsyncStorage>(value)
}

pub fn use_store<T: Storable>(init: impl Fn() -> T) -> Store<T, MappedMutSignal<T, CopyValue<T>>>
where
    Store<T, MappedMutSignal<T, CopyValue<T>>>: Clone,
{
    use_hook(move || create_store(init()))
}

pub trait Storable {
    type Store<View: Writable<Target = Self>>;

    fn create_selector<View: Writable<Target = Self>>(
        selector: SelectorScope<View>,
    ) -> Self::Store<View>;
}
