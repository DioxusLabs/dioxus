//! Additional utilities for indexing into stores.

use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    ops::{self, Index, IndexMut},
};

use crate::{store::Store, ProjectHashExt, ProjectPath, ReadStore};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, ReadSignal, Readable, UnsyncStorage, Writable,
    WriteLock, WriteSignal,
};

mod sealed {
    pub trait Sealed<Idx, P> {}
}

/// The way a data structure scopes a projector to one of its indexed children.
#[doc(hidden)]
pub trait IndexSelector<Idx, P>: sealed::Sealed<Idx, P> {
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

impl<T, P> sealed::Sealed<usize, P> for Vec<T> where P: ProjectPath {}

impl<T, P> IndexSelector<usize, P> for [T]
where
    P: ProjectPath,
{
    fn scope_project(project: P, index: &usize) -> P {
        project.project_key(*index as _)
    }
}

impl<T, P> sealed::Sealed<usize, P> for [T] where P: ProjectPath {}

impl<K, V, I, P> IndexSelector<I, P> for HashMap<K, V>
where
    I: Hash,
    P: ProjectPath,
{
    fn scope_project(project: P, index: &I) -> P {
        project.project_hash_key(index)
    }
}

impl<K, V, I, P> sealed::Sealed<I, P> for HashMap<K, V>
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

impl<K, V, I, P> sealed::Sealed<I, P> for BTreeMap<K, V>
where
    I: Hash,
    P: ProjectPath,
{
}

/// A specific index in a `Readable` / `Writable` type
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
    Write::Target: ops::Index<Index> + 'static,
    Index: Clone,
{
    type Target = <Write::Target as ops::Index<Index>>::Output;

    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().map(|value| {
            Self::Storage::map(value, |value: &Write::Target| {
                value.index(self.index.clone())
            })
        })
    }

    fn try_peek_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().map(|value| {
            Self::Storage::map(value, |value: &Write::Target| {
                value.index(self.index.clone())
            })
        })
    }

    fn subscribers(&self) -> dioxus_core::Subscribers
    where
        Self::Target: 'static,
    {
        self.write.subscribers()
    }
}

impl<Index, Write> Writable for IndexWrite<Index, Write>
where
    Write: Writable,
    Write::Target: ops::IndexMut<Index> + 'static,
    Index: Clone,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> Result<dioxus_signals::WritableRef<'static, Self>, BorrowMutError>
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

impl<Idx, T, Write> ::std::convert::From<Store<T, IndexWrite<Idx, Write>>>
    for Store<T, WriteSignal<T>>
where
    Write: Writable<Storage = UnsyncStorage> + 'static,
    Write::WriteMetadata: 'static,
    Write::Target: ops::IndexMut<Idx, Output = T> + 'static,
    Idx: Clone + 'static,
    T: 'static,
{
    fn from(value: Store<T, IndexWrite<Idx, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| WriteSignal::new(writer))
            .into()
    }
}

impl<Idx, T, Write> ::std::convert::From<Store<T, IndexWrite<Idx, Write>>> for ReadStore<T>
where
    Write: Readable<Storage = UnsyncStorage> + 'static,
    Write::Target: ops::Index<Idx, Output = T> + 'static,
    Idx: Clone + 'static,
    T: 'static,
{
    fn from(value: Store<T, IndexWrite<Idx, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| ReadSignal::new(writer))
            .into()
    }
}
