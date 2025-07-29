use crate::{SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable, WriteSignal};
use std::marker::PhantomData;

impl<T> Storable for [T] {
    type Store<View: Writable<Target = Self>> = SliceSelector<View, T>;

    fn create_selector<View: Writable<Target = Self>>(
        selector: SelectorScope<View>,
    ) -> Self::Store<View> {
        SliceSelector::new(selector)
    }
}

pub struct SliceSelector<W, T> {
    selector: SelectorScope<W>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T> PartialEq for SliceSelector<W, T>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, T> Clone for SliceSelector<W, T>
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

impl<W, T> Copy for SliceSelector<W, T> where W: Copy {}

impl<
        T,
        W: Writable<Storage = UnsyncStorage> + 'static,
        F: Fn(&W::Target) -> &[T] + 'static,
        FMut: Fn(&mut W::Target) -> &mut [T] + 'static,
    > ::std::convert::From<SliceSelector<MappedMutSignal<[T], W, F, FMut>, T>>
    for SliceSelector<WriteSignal<[T]>, T>
{
    fn from(value: SliceSelector<MappedMutSignal<[T], W, F, FMut>, T>) -> Self {
        SliceSelector {
            selector: value.selector.map(::std::convert::Into::into),
            _phantom: PhantomData,
        }
    }
}

impl<W, T> SliceSelector<W, T> {
    fn new(selector: SelectorScope<W>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<W: Writable<Target = [T]> + Copy + 'static, T: Storable + 'static> SliceSelector<W, T> {
    pub fn index(
        self,
        index: usize,
    ) -> Store<
        T,
        MappedMutSignal<
            T,
            W,
            impl Fn(&[T]) -> &T + Copy + 'static,
            impl Fn(&mut [T]) -> &mut T + Copy + 'static,
        >,
    > {
        T::create_selector(self.selector.scope(
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
                impl Fn(&[T]) -> &T + Copy + 'static,
                impl Fn(&mut [T]) -> &mut T + Copy + 'static,
            >,
        >,
    > {
        (0..self.len()).map(move |i| self.index(i))
    }
}
