use crate::{CreateSelector, SelectorScope, SelectorStorage, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable};
use std::marker::PhantomData;

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

    pub fn into_iter(
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

    pub fn remove(self, index: u32) -> T {
        self.selector.mark_dirty_shallow();
        self.selector.mark_dirty_at_and_after_index(index as usize);
        self.selector.write.write_unchecked().remove(index as usize)
    }

    pub fn insert(self, index: u32, value: T) {
        self.selector.mark_dirty_shallow();
        self.selector.mark_dirty_at_and_after_index(index as usize);
        self.selector
            .write
            .write_unchecked()
            .insert(index as usize, value);
    }

    pub fn clear(self) {
        self.selector.mark_dirty();
        self.selector.write.write_unchecked().clear();
    }

    pub fn retain(self, mut f: impl FnMut(&T) -> bool) {
        let mut index = 0;
        let mut first_removed_index = None;
        self.selector.write.write_unchecked().retain(|item| {
            let keep = f(item);
            if !keep {
                first_removed_index = first_removed_index.or(Some(index));
            }
            index += 1;
            keep
        });
        if let Some(index) = first_removed_index {
            self.selector.mark_dirty_shallow();
            self.selector.mark_dirty_at_and_after_index(index);
        }
    }
}
