//! Store-specific boxed-signal conversions for `HashMap` projections.

use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

use crate::{store::Store, ReadStore};
use dioxus_signals::project::HashMapGetWrite;
use dioxus_signals::{ReadSignal, Readable, UnsyncStorage, Writable, WriteSignal};

impl<Index, Write, K, V, St> From<Store<V, HashMapGetWrite<Index, Write>>>
    for Store<V, WriteSignal<V>>
where
    Write::WriteMetadata: 'static,
    Write: Writable<Target = HashMap<K, V, St>, Storage = UnsyncStorage> + 'static,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
    V: 'static,
{
    fn from(value: Store<V, HashMapGetWrite<Index, Write>>) -> Self {
        value.into_selector().map_writer(WriteSignal::new).into()
    }
}

impl<Index, Write, K, V, St> From<Store<V, HashMapGetWrite<Index, Write>>> for ReadStore<V>
where
    Write: Readable<Target = HashMap<K, V, St>, Storage = UnsyncStorage> + 'static,
    Index: Hash + Eq + 'static,
    K: Borrow<Index> + Eq + Hash + 'static,
    St: BuildHasher + 'static,
    V: 'static,
{
    fn from(value: Store<V, HashMapGetWrite<Index, Write>>) -> Self {
        value.into_selector().map_writer(ReadSignal::new).into()
    }
}
