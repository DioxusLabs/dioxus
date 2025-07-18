#![allow(clippy::type_complexity)]

use std::hash::Hash;

use crate::subscriptions::{StoreSubscriptions, StoreSubscriptionsInner, TinyVec};
use dioxus_core::use_hook;
use dioxus_signals::{
    BorrowError, BorrowMutError, CopyValue, MappedMutSignal, Readable, ReadableRef, Storage,
    Subscribers, UnsyncStorage, Writable, WritableExt, WritableRef, WriteSignal,
};

mod foreign;
pub use foreign::*;
mod hashmap;
mod subscriptions;
mod vec;
pub use hashmap::*;
pub use vec::*;

// Re-exported for the macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_signals;
}

#[allow(private_bounds)]
pub trait SelectorStorage: Storage<StoreSubscriptionsInner> {}
impl<S: Storage<StoreSubscriptionsInner>> SelectorStorage for S {}

pub struct SelectorScope<W, S: SelectorStorage = UnsyncStorage> {
    path: TinyVec,
    store: StoreSubscriptions<S>,
    write: W,
}

impl<W: PartialEq, S: SelectorStorage> PartialEq for SelectorScope<W, S> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.write == other.write
    }
}

impl<W, S: SelectorStorage> Clone for SelectorScope<W, S>
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

impl<W, S: SelectorStorage> Copy for SelectorScope<W, S> where W: Copy {}

impl<W, S: SelectorStorage> SelectorScope<W, S> {
    fn new(path: TinyVec, store: StoreSubscriptions<S>, write: W) -> Self {
        Self { path, store, write }
    }

    pub fn hash_scope<U: 'static, F, FMut>(
        self,
        index: impl Hash,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, W, F, FMut>, S>
    where
        W: Writable<Storage = S> + Copy + 'static,
        F: Fn(&W::Target) -> &U + Copy + 'static,
        FMut: Fn(&mut W::Target) -> &mut U + Copy + 'static,
    {
        let hash = self.store.hash(index);
        self.scope(hash, map, map_mut)
    }

    pub fn scope<U: 'static, F, FMut>(
        self,
        index: u32,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, W, F, FMut>, S>
    where
        W: Writable<Storage = S> + Copy + 'static,
        F: Fn(&W::Target) -> &U + Copy + 'static,
        FMut: Fn(&mut W::Target) -> &mut U + Copy + 'static,
    {
        let Self {
            mut path,
            store,
            write,
        } = self;
        path.push(index);
        let write = write.map_mut(map, map_mut);
        SelectorScope::new(path, store, write)
    }

    fn track(&self) {
        self.store.track(&self.path);
    }

    fn track_nested(&self) {
        self.store.track_nested(&self.path);
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
    pub fn map<W2>(self, map: impl FnOnce(W) -> W2) -> SelectorScope<W2, S> {
        SelectorScope {
            path: self.path,
            store: self.store,
            write: map(self.write),
        }
    }
}

impl<W: Readable, S: SelectorStorage> SelectorScope<W, S> {
    pub fn try_read_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.track_nested();
        self.write.try_read_unchecked()
    }

    pub fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.write.try_peek_unchecked()
    }

    pub fn subscribers(&self) -> Option<Subscribers> {
        self.store.subscribers(&self.path)
    }
}

impl<W: Writable, S: SelectorStorage> SelectorScope<W, S> {
    pub fn try_write_unchecked(&self) -> Result<WritableRef<'static, W>, BorrowMutError> {
        self.mark_dirty();
        self.write.try_write_unchecked()
    }
}

pub type Store<T, W = WriteSignal<T>, S = UnsyncStorage> = <T as Storable>::Store<W, S>;

pub fn create_maybe_sync_store<T: Storable, S: SelectorStorage + Storage<T>>(
    value: T,
) -> Store<T, MappedMutSignal<T, CopyValue<T, S>>, S> {
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
    T::Store::new(selector)
}

pub fn use_maybe_sync_store<T: Storable, S: SelectorStorage + Storage<T>>(
    init: impl Fn() -> T,
) -> Store<T, MappedMutSignal<T, CopyValue<T, S>>, S>
where
    Store<T, MappedMutSignal<T, CopyValue<T, S>>, S>: Clone,
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
    type Store<View, S: SelectorStorage>: CreateSelector<View = View, Storage = S>;
}

pub trait CreateSelector {
    type View;
    type Storage: SelectorStorage;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self;
}
