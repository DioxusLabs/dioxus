use std::ops::DerefMut;
use std::ops::IndexMut;

use crate::store::Store;
use crate::IndexStoreExt;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

pub trait SliceStoreExt {
    type Slice;
    type Item;
    type Write;

    fn len(self) -> usize;

    fn is_empty(self) -> bool;

    fn iter(
        self,
    ) -> impl Iterator<
        Item = Store<
            Self::Item,
            MappedMutSignal<
                Self::Item,
                Self::Write,
                impl Fn(&Self::Slice) -> &Self::Item + Copy + 'static,
                impl Fn(&mut Self::Slice) -> &mut Self::Item + Copy + 'static,
            >,
        >,
    >;
}

impl<W, T, I> SliceStoreExt for Store<T, W>
where
    W: Writable<Target = T> + Copy + 'static,
    T: DerefMut<Target = [I]> + IndexMut<usize, Output = I> + 'static,
    I: 'static,
{
    type Slice = T;
    type Item = I;
    type Write = W;

    fn len(self) -> usize {
        self.selector().track();
        self.selector().write.read().deref().len()
    }

    fn is_empty(self) -> bool {
        self.selector().track();
        self.selector().write.read().deref().is_empty()
    }

    fn iter(
        self,
    ) -> impl Iterator<
        Item = Store<
            I,
            MappedMutSignal<
                I,
                W,
                impl Fn(&T) -> &I + Copy + 'static,
                impl Fn(&mut T) -> &mut I + Copy + 'static,
            >,
        >,
    > {
        (0..self.len()).map(move |i| IndexStoreExt::index(self, i))
    }
}
