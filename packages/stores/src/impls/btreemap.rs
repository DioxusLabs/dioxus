//! `Store<BTreeMap<K, V>, _>` — shape-agnostic methods live on the
//! [`ProjectBTreeMap`](crate::ProjectBTreeMap) /
//! [`ProjectBTreeMapMut`](crate::ProjectBTreeMapMut) traits. The store-specific
//! accessors now also live on [`ProjectBTreeMap`]. This module keeps the
//! `GetWrite` lens and store-specific boxed-signal conversions.

use std::{borrow::Borrow, collections::BTreeMap, panic::Location};

use crate::{store::Store, ReadStore};
use dioxus_signals::{
    AnyStorage, BorrowError, BorrowMutError, ReadSignal, Readable, UnsyncStorage, Writable,
    WriteLock, WriteSignal,
};
use generational_box::ValueDroppedError;

/// A specific index in a `Readable` / `Writable` BTreeMap.
#[derive(Clone, Copy)]
pub struct GetWrite<Index, Write> {
    index: Index,
    write: Write,
    created: &'static Location<'static>,
}

impl<Index, Write> GetWrite<Index, Write> {
    pub(crate) fn new(index: Index, write: Write, created: &'static Location<'static>) -> Self {
        Self {
            index,
            write,
            created,
        }
    }
}

impl<Index, Write, K, V> Readable for GetWrite<Index, Write>
where
    Write: Readable<Target = BTreeMap<K, V>>,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
{
    type Target = V;
    type Storage = Write::Storage;

    fn try_read_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_read_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
                .ok_or_else(|| BorrowError::Dropped(ValueDroppedError::new(self.created)))
        })
    }

    fn try_peek_unchecked(&self) -> Result<dioxus_signals::ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.write.try_peek_unchecked().and_then(|value| {
            Self::Storage::try_map(value, |value: &Write::Target| value.get(&self.index))
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

impl<Index, Write, K, V> Writable for GetWrite<Index, Write>
where
    Write: Writable<Target = BTreeMap<K, V>>,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
{
    type WriteMetadata = Write::WriteMetadata;

    fn try_write_unchecked(
        &self,
    ) -> Result<dioxus_signals::WritableRef<'static, Self>, BorrowMutError>
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

impl<Index, Write, K, V> ::std::convert::From<Store<V, GetWrite<Index, Write>>>
    for Store<V, WriteSignal<V>>
where
    Write::WriteMetadata: 'static,
    Write: Writable<Target = BTreeMap<K, V>, Storage = UnsyncStorage> + 'static,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
    V: 'static,
{
    fn from(value: Store<V, GetWrite<Index, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| WriteSignal::new(writer))
            .into()
    }
}

impl<Index, Write, K, V> ::std::convert::From<Store<V, GetWrite<Index, Write>>> for ReadStore<V>
where
    Write: Readable<Target = BTreeMap<K, V>, Storage = UnsyncStorage> + 'static,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
    V: 'static,
{
    fn from(value: Store<V, GetWrite<Index, Write>>) -> Self {
        value
            .into_selector()
            .map_writer(|writer| ReadSignal::new(writer))
            .into()
    }
}
