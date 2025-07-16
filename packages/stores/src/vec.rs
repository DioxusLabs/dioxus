use std::marker::PhantomData;

use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable};

use crate::{CreateSelector, SelectorScope, SelectorStorage, Storable, Store};

impl<T> Storable for Vec<T> {
    type Store<View, S: SelectorStorage> = VecSelector<View, T, S>;
}

pub struct VecSelector<W, T, S: SelectorStorage = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T, S: SelectorStorage> Clone for VecSelector<W, T, S>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<W, T, S: SelectorStorage> Copy for VecSelector<W, T, S> where W: Copy {}

impl<W, T, S: SelectorStorage> CreateSelector for VecSelector<W, T, S> {
    type View = W;
    type Storage = S;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<
        W: Writable<Target = Vec<T>, Storage = S> + Copy + 'static,
        T: Storable + 'static,
        S: SelectorStorage,
    > VecSelector<W, T, S>
{
    pub fn index(
        self,
        index: u32,
    ) -> Store<
        T,
        MappedMutSignal<
            T,
            W,
            impl Fn(&Vec<T>) -> &T + Copy + 'static,
            impl Fn(&mut Vec<T>) -> &mut T + Copy + 'static,
        >,
        S,
    > {
        T::Store::new(self.selector.scope(
            index,
            move |value| &value[index as usize],
            move |value| &mut value[index as usize],
        ))
    }

    pub fn len(self) -> usize {
        self.selector.track();
        self.selector.write.read().len()
    }

    pub fn is_empty(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_empty()
    }

    pub fn iter(
        self,
    ) -> impl Iterator<
        Item = Store<
            T,
            MappedMutSignal<
                T,
                W,
                impl Fn(&Vec<T>) -> &T + Copy + 'static,
                impl Fn(&mut Vec<T>) -> &mut T + Copy + 'static,
            >,
            S,
        >,
    > {
        (0..self.len()).map(move |i| self.index(i as u32))
    }

    pub fn push(self, value: T) {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().push(value);
    }
}
