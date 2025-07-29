use std::{hash::Hash, ops::IndexMut};

use crate::store::Store;
use dioxus_signals::{MappedMutSignal, Writable};

pub trait IndexStoreExt<Idx> {
    type Collection;
    type Write;
    type Item;

    fn index(
        self,
        index: Idx,
    ) -> Store<
        Self::Item,
        MappedMutSignal<
            Self::Item,
            Self::Write,
            impl Fn(&Self::Collection) -> &Self::Item + Copy + 'static,
            impl Fn(&mut Self::Collection) -> &mut Self::Item + Copy + 'static,
        >,
    >;
}

impl<W, T, I, Idx> IndexStoreExt<Idx> for Store<T, W>
where
    W: Writable<Target = T> + Copy + 'static,
    T: IndexMut<Idx, Output = I> + 'static,
    I: 'static,
    Idx: Hash + Copy + 'static,
{
    type Collection = T;
    type Write = W;
    type Item = I;

    fn index(
        self,
        index: Idx,
    ) -> Store<
        Self::Item,
        MappedMutSignal<
            Self::Item,
            W,
            impl Fn(&T) -> &Self::Item + Copy + 'static,
            impl Fn(&mut T) -> &mut Self::Item + Copy + 'static,
        >,
    > {
        Store::new(self.selector().hash_scope(
            index,
            move |value| value.index(index),
            move |value| value.index_mut(index),
        ))
    }
}
