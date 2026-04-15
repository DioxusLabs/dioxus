//! Store-specific boxed-signal conversions for indexed projections.

use std::ops::{Index, IndexMut};

use crate::{store::Store, ReadStore};
use dioxus_signals::project::IndexWrite;
use dioxus_signals::{ReadSignal, Readable, UnsyncStorage, Writable, WriteSignal};

impl<Idx, T, Write> From<Store<T, IndexWrite<Idx, Write>>> for Store<T, WriteSignal<T>>
where
    Write: Writable<Storage = UnsyncStorage> + 'static,
    Write::WriteMetadata: 'static,
    Write::Target: IndexMut<Idx, Output = T> + 'static,
    Idx: Clone + 'static,
    T: 'static,
{
    fn from(value: Store<T, IndexWrite<Idx, Write>>) -> Self {
        value.into_selector().map_writer(WriteSignal::new).into()
    }
}

impl<Idx, T, Write> From<Store<T, IndexWrite<Idx, Write>>> for ReadStore<T>
where
    Write: Readable<Storage = UnsyncStorage> + 'static,
    Write::Target: Index<Idx, Output = T> + 'static,
    Idx: Clone + 'static,
    T: 'static,
{
    fn from(value: Store<T, IndexWrite<Idx, Write>>) -> Self {
        value.into_selector().map_writer(ReadSignal::new).into()
    }
}
