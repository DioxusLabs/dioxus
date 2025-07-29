use crate::{CreateSelector, SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable, WriteSignal};
use std::marker::PhantomData;

impl<T> Storable for Vec<T> {
    type Store<View> = VecSelector<View, T>;
}

pub struct VecSelector<W, T> {
    selector: SelectorScope<W>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T> PartialEq for VecSelector<W, T>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, T> Clone for VecSelector<W, T>
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

impl<W, T> Copy for VecSelector<W, T> where W: Copy {}

impl<
        T,
        W: Writable<Storage = UnsyncStorage> + 'static,
        F: Fn(&W::Target) -> &Vec<T> + 'static,
        FMut: Fn(&mut W::Target) -> &mut Vec<T> + 'static,
    > ::std::convert::From<VecSelector<MappedMutSignal<Vec<T>, W, F, FMut>, T>>
    for VecSelector<WriteSignal<Vec<T>>, T>
{
    fn from(value: VecSelector<MappedMutSignal<Vec<T>, W, F, FMut>, T>) -> Self {
        VecSelector {
            selector: value.selector.map(::std::convert::Into::into),
            _phantom: PhantomData,
        }
    }
}

impl<W, T> CreateSelector for VecSelector<W, T> {
    type View = W;

    fn new(selector: SelectorScope<Self::View>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<W: Writable<Target = Vec<T>> + Copy + 'static, T: Storable + 'static> VecSelector<W, T> {
    pub fn index(
        self,
        index: usize,
    ) -> Store<
        T,
        MappedMutSignal<
            T,
            W,
            impl Fn(&Vec<T>) -> &T + Copy + 'static,
            impl Fn(&mut Vec<T>) -> &mut T + Copy + 'static,
        >,
    > {
        T::Store::new(self.selector.scope(
            index as u32,
            move |value| &value[index],
            move |value| &mut value[index],
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
        >,
    > {
        (0..self.len()).map(move |i| self.index(i))
    }

    pub fn push(self, value: T) {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().push(value);
    }

    pub fn remove(self, index: usize) -> T {
        self.selector.mark_dirty_shallow();
        self.selector.mark_dirty_at_and_after_index(index);
        self.selector.write.write_unchecked().remove(index)
    }

    pub fn insert(self, index: usize, value: T) {
        self.selector.mark_dirty_shallow();
        self.selector.mark_dirty_at_and_after_index(index);
        self.selector.write.write_unchecked().insert(index, value);
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
