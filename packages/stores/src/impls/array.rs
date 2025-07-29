use crate::{SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable, WriteSignal};
use std::marker::PhantomData;

impl<const N: usize, T> Storable for [T; N] {
    type Store<View: Writable<Target = Self>> = ArraySelector<View, T>;

    fn create_selector<View: Writable<Target = Self>>(
        selector: SelectorScope<View>,
    ) -> Self::Store<View> {
        ArraySelector::new(selector)
    }
}

pub struct ArraySelector<W, T> {
    selector: SelectorScope<W>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T> PartialEq for ArraySelector<W, T>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, T> Clone for ArraySelector<W, T>
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

impl<W, T> Copy for ArraySelector<W, T> where W: Copy {}

impl<
        const N: usize,
        T,
        W: Writable<Storage = UnsyncStorage> + 'static,
        F: Fn(&W::Target) -> &[T; N] + 'static,
        FMut: Fn(&mut W::Target) -> &mut [T; N] + 'static,
    > ::std::convert::From<ArraySelector<MappedMutSignal<[T; N], W, F, FMut>, T>>
    for ArraySelector<WriteSignal<[T; N]>, T>
{
    fn from(value: ArraySelector<MappedMutSignal<[T; N], W, F, FMut>, T>) -> Self {
        ArraySelector {
            selector: value.selector.map(::std::convert::Into::into),
            _phantom: PhantomData,
        }
    }
}

impl<W, T> ArraySelector<W, T> {
    fn new(selector: SelectorScope<W>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<const N: usize, W: Writable<Target = [T; N]> + Copy + 'static, T: Storable + 'static>
    ArraySelector<W, T>
{
    pub fn index(
        self,
        index: usize,
    ) -> Store<
        T,
        MappedMutSignal<
            T,
            W,
            impl Fn(&[T; N]) -> &T + Copy + 'static,
            impl Fn(&mut [T; N]) -> &mut T + Copy + 'static,
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
                impl Fn(&[T; N]) -> &T + Copy + 'static,
                impl Fn(&mut [T; N]) -> &mut T + Copy + 'static,
            >,
        >,
    > {
        (0..self.len()).map(move |i| self.index(i))
    }
}
