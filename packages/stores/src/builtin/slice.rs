use crate::{store_impls, SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable, WriteSignal};
use std::{marker::PhantomData, ops::DerefMut};

impl<T> Storable for [T] {
    type Store<View: Writable<Target = Self>> = SliceSelector<View, [T]>;

    fn create_selector<View: Writable<Target = Self>>(
        selector: SelectorScope<View>,
    ) -> Self::Store<View> {
        SliceSelector::new(selector)
    }
}

impl<const N: usize, T> Storable for [T; N] {
    type Store<View: Writable<Target = Self>> = SliceSelector<View, [T; N]>;

    fn create_selector<View: Writable<Target = Self>>(
        selector: SelectorScope<View>,
    ) -> Self::Store<View> {
        SliceSelector::new(selector)
    }
}

pub struct SliceSelector<W, T: ?Sized> {
    selector: SelectorScope<W>,
    _phantom: std::marker::PhantomData<Box<T>>,
}

store_impls!(T => SliceSelector<W, T>);

impl<W, T: ?Sized> SliceSelector<W, T> {
    fn new(selector: SelectorScope<W>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<W: Writable<Target = T> + Copy + 'static, I: Storable + 'static, T: DerefMut<Target=[I]>  + 'static> SliceSelector<W, T> {
    pub fn index(
        self,
        index: usize,
    ) -> Store<
        I,
        MappedMutSignal<
            I,
            W,
            impl Fn(&T) -> &I + Copy + 'static,
            impl Fn(&mut T) -> &mut I + Copy + 'static,
        >,
    > {
        I::create_selector(self.selector.scope(
            index as u32,
            move |value| &value.deref()[index],
            move |value| &mut value.deref_mut()[index],
        ))
    }

    pub fn len(self) -> usize {
        self.selector.track();
        self.selector.write.read().deref().len()
    }

    pub fn is_empty(self) -> bool {
        self.selector.track();
        self.selector.write.read().deref().is_empty()
    }

    pub fn iter(
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
        (0..self.len()).map(move |i| self.index(i))
    }
}
