//! Store-specific boxed-signal conversions for `BTreeMap` projections.

use std::{borrow::Borrow, collections::BTreeMap};

use crate::{store::Store, ReadStore};
use dioxus_signals::project::BTreeMapGetWrite;
use dioxus_signals::{ReadSignal, Readable, UnsyncStorage, Writable, WriteSignal};

impl<Index, Write, K, V> From<Store<V, BTreeMapGetWrite<Index, Write>>> for Store<V, WriteSignal<V>>
where
    Write::WriteMetadata: 'static,
    Write: Writable<Target = BTreeMap<K, V>, Storage = UnsyncStorage> + 'static,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
    V: 'static,
{
    fn from(value: Store<V, BTreeMapGetWrite<Index, Write>>) -> Self {
        value.into_selector().map_writer(WriteSignal::new).into()
    }
}

impl<Index, Write, K, V> From<Store<V, BTreeMapGetWrite<Index, Write>>> for ReadStore<V>
where
    Write: Readable<Target = BTreeMap<K, V>, Storage = UnsyncStorage> + 'static,
    Index: Ord + 'static,
    K: Borrow<Index> + Ord + 'static,
    V: 'static,
{
    fn from(value: Store<V, BTreeMapGetWrite<Index, Write>>) -> Self {
        value.into_selector().map_writer(ReadSignal::new).into()
    }
}
