//! Additional utilities for `Vec` stores.

use std::{iter::FusedIterator, panic::Location};

use crate::{impls::index::IndexSelector, store::Store, ReadStore};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, ReadSignal, Readable, ReadableExt, UnsyncStorage,
    Writable, WriteLock, WriteSignal,
};
use generational_box::ValueDroppedError;

impl<Lens, I> Store<Vec<I>, Lens>
where
    Lens: Readable<Target = Vec<I>> + 'static,
    I: 'static,
{
    /// Returns the length of the slice. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// assert_eq!(store.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.selector().track_shallow();
        self.selector().peek().len()
    }

    /// Checks if the slice is empty. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// assert!(!store.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.selector().track_shallow();
        self.selector().peek().is_empty()
    }

    /// Returns an iterator over the items in the slice. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// for item in store.iter() {
    ///     println!("{}", item);
    /// }
    /// ```
    #[track_caller]
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = Store<I, VecGetWrite<Lens>>>
           + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        Lens: Clone,
    {
        let location = Location::caller();
        (0..self.len()).map(move |i| self.clone().get_unchecked_at(i, location))
    }

    /// Try to get an item from slice. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change. The new store
    /// will only update when the item at the index changes.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// let indexed_store = store.get(1).unwrap();
    /// // The indexed store can access the store methods of the indexed store.
    /// assert_eq!(indexed_store(), 2);
    /// ```
    pub fn get(&self, index: usize) -> Option<Store<I, VecGetWrite<Lens>>>
    where
        Lens: Clone,
    {
        if index >= self.len() {
            None
        } else {
            Some(self.clone().get_unchecked(index))
        }
    }

    /// Get a store for the item at the given index without checking if it is in bounds.
    ///
    /// This is not unsafe, but reads will return a [BorrowError::Dropped] error if the index is out of bounds.
    #[track_caller]
    pub fn get_unchecked(self, index: usize) -> Store<I, VecGetWrite<Lens>> {
        self.get_unchecked_at(index, Location::caller())
    }

    fn get_unchecked_at(
        self,
        index: usize,
        location: &'static Location<'static>,
    ) -> Store<I, VecGetWrite<Lens>> {
        <Vec<I>>::scope_selector(self.into_selector(), &index)
            .map_writer(move |write| VecGetWrite {
                index,
                write,
                created: location,
            })
            .into()
    }
}

/// A specific index in a `Readable` / `Writable` Vec that uses safe `.get()` / `.get_mut()` access.
#[derive(Clone, Copy)]
pub struct VecGetWrite<Write> {
    index: usize,
    write: Write,
    created: &'static Location<'static>,
}

impl<Write, T> Readable for VecGetWrite<Write>
where
    Write: Readable<Target = Vec<T>>,
    T: 'static,
{
    type Target = T;
    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().and_then(|value| {
            let index = self.index;
            Self::Storage::try_map(value, move |value: &Vec<T>| value.get(index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn try_peek_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().and_then(|value| {
            let index = self.index;
            Self::Storage::try_map(value, move |value: &Vec<T>| value.get(index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn subscribers(&self) -> dioxus_core::Subscribers
    where
        Self::Target: 'static,
    {
        self.write.subscribers()
    }
}

impl<Write, T> Writable for VecGetWrite<Write>
where
    Write: Writable<Target = Vec<T>>,
    T: 'static,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> Result<dioxus_signals::WritableRef<'static, Self>, BorrowMutError>
    where
        Self::Target: 'static,
    {
        self.write.try_write_unchecked().and_then(|value| {
            let index = self.index;
            WriteLock::filter_map(value, move |value: &mut Vec<T>| value.get_mut(index))
                .ok_or_else(|| BorrowMutError::Dropped(ValueDroppedError::new(self.created)))
        })
    }
}

impl<T, Write> ::std::convert::From<Store<T, VecGetWrite<Write>>> for Store<T, WriteSignal<T>>
where
    Write: Writable<Target = Vec<T>, Storage = UnsyncStorage> + 'static,
    Write::WriteMetadata: 'static,
    T: 'static,
{
    fn from(value: Store<T, VecGetWrite<Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| WriteSignal::new(writer))
            .into()
    }
}

impl<T, Write> ::std::convert::From<Store<T, VecGetWrite<Write>>> for ReadStore<T>
where
    Write: Readable<Target = Vec<T>, Storage = UnsyncStorage> + 'static,
    T: 'static,
{
    fn from(value: Store<T, VecGetWrite<Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| ReadSignal::new(writer))
            .into()
    }
}
